use super::*;

fn binding() -> Result<Bindings, String> {
    Bindings::parse(include_bytes!("fixtures/talk.json"))
}

fn canonical_inputs() -> Result<(Bindings, Value, Value), String> {
    let mut bindings = binding()?;
    let recipe = serde_json::json!({"stages":[{"key":"native","count":1}],"summary":"Canonical chosen fixture"});
    bindings.quests[0].recipe_sha256 =
        Sha256::digest(serde_json::to_vec(&recipe).map_err(|error| error.to_string())?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
    let index = serde_json::json!({"schema":"OTERYN_FAMILY_INDEX/v1","family":"Quest","shards":["fixture-shard"],"record_count":1});
    let shard = serde_json::json!({"schema":"OTERYN_QUEST_TREE_SHARD/v1","family":"Quest","records":[{"definition":{
        "identity":{"key":bindings.quests[0].quest,"revision":"authored-r1"},"definition_profile":"oteryn_authored_v1",
        "data_complete":true,"missing_data":[{"code":"quest_native_lowering_missing"},{"code":"authored_trigger_and_delivery_bindings_missing"}],"recipe":recipe
    }}]});
    Ok((bindings, index, shard))
}

fn evidence(index: &Value, shard: &Value) -> Result<RecipeEvidence, String> {
    RecipeEvidence::from_documents(&index.to_string(), &[("fixture-shard", &shard.to_string())])
}

#[test]
fn imported_chosen_recipe_admission_preserves_original_source_holds() -> Result<(), String> {
    let (mut bindings, index, mut shard) = canonical_inputs()?;
    let quest = "oteryn:quest.in_service_of_yalahar_quest";
    bindings.quests[0].quest = quest.into();
    for stage in &mut bindings.quests[0].stages {
        let super::super::Trigger::NpcDialogue { callback } = &mut stage.trigger else {
            return Err("expected NPC dialogue fixture".into());
        };
        callback.quest = quest.into();
    }
    let definition = &mut shard["records"][0]["definition"];
    definition["identity"]["key"] = quest.into();
    let recipe = definition["recipe"].take();
    let identity = definition["identity"].clone();
    definition
        .as_object_mut()
        .ok_or("definition")?
        .remove("definition_profile");
    definition
        .as_object_mut()
        .ok_or("definition")?
        .remove("data_complete");
    definition["missing_data"] = serde_json::json!([
        {"code":"reported_source_gap","field":"interactions_with_gaps","count":14},
        {"code":"quest_native_lowering_missing"}
    ]);
    definition["oteryn_recipe"] = serde_json::json!({
        "profile":"chosen_source_completion_v1",
        "chosen_data_complete":true,
        "source_fidelity":"ORIGINAL_HOLDS_PRESERVED",
        "payload":{"identity":identity,"recipe":recipe}
    });
    evidence(&index, &shard)?.verify(&bindings)?;
    for mutation in 0..5 {
        let mut refused = shard.clone();
        let definition = &mut refused["records"][0]["definition"];
        match mutation {
            0 => definition["oteryn_recipe"]["chosen_data_complete"] = false.into(),
            1 => definition["oteryn_recipe"]["profile"] = "unregistered_profile".into(),
            2 => {
                definition["oteryn_recipe"]["payload"]["identity"]["key"] =
                    "oteryn:quest.foreign".into()
            }
            3 => {
                definition["missing_data"] =
                    serde_json::json!([{"code":"chosen_recipe_data_missing"}])
            }
            _ => {
                definition["oteryn_recipe"]["payload"]["recipe"]["summary"] =
                    "Changed after binding".into()
            }
        }
        assert!(
            evidence(&index, &refused)
                .and_then(|proof| proof.verify(&bindings))
                .is_err(),
            "imported admission accepted mutation {mutation}"
        );
    }
    Ok(())
}
