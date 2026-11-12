use super::*;

fn binding() -> Result<Bindings, String> {
    Bindings::parse(include_bytes!("fixtures/kill.json"))
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
fn canonical_admission_rejects_stale_recipes_and_unresolved_data_holds() -> Result<(), String> {
    let (bindings, index, shard) = canonical_inputs()?;
    evidence(&index, &shard)?.verify(&bindings)?;
    let mut stale = bindings.clone();
    stale.quests[0].recipe_sha256 = "f".repeat(64);
    assert!(
        evidence(&index, &shard)?
            .verify(&stale)
            .is_err_and(|error| error.contains("recipe digest differs"))
    );
    let mut held = shard.clone();
    held["records"][0]["definition"]["missing_data"][0]["code"] = "requirement_unknown".into();
    assert!(
        evidence(&index, &held)?
            .verify(&bindings)
            .is_err_and(|error| error.contains("unresolved data holds"))
    );
    held = shard;
    held["records"][0]["definition"]["data_complete"] = false.into();
    assert!(
        evidence(&index, &held)?
            .verify(&bindings)
            .is_err_and(|error| error.contains("complete chosen recipe"))
    );
    Ok(())
}

#[test]
fn canonical_shard_membership_and_population_are_exact() -> Result<(), String> {
    let (_, index, shard) = canonical_inputs()?;
    let mut wrong = index.clone();
    wrong["shards"][0] = "missing-shard".into();
    assert!(evidence(&wrong, &shard).is_err());
    wrong = index.clone();
    wrong["shards"]
        .as_array_mut()
        .ok_or("shards")?
        .push("fixture-shard".into());
    assert!(evidence(&wrong, &shard).is_err());
    wrong = index;
    wrong["record_count"] = 2.into();
    assert!(evidence(&wrong, &shard).is_err());
    let mut duplicated = shard;
    let row = duplicated["records"][0].clone();
    duplicated["records"]
        .as_array_mut()
        .ok_or("records")?
        .push(row);
    assert!(evidence(&wrong, &duplicated).is_err_and(|error| error.contains("duplicate")));
    Ok(())
}

#[test]
fn binding_document_union_refuses_duplicate_owners_and_unknown_fields() -> Result<(), String> {
    let bindings = binding()?;
    let source = serde_json::to_string(&bindings).map_err(|error| error.to_string())?;
    assert_eq!(Bindings::parse_documents(&[&source])?, bindings);
    assert!(Bindings::parse_documents(&[&source, &source]).is_err());
    let mut unknown: Value = serde_json::from_str(&source).map_err(|error| error.to_string())?;
    unknown["runtime_qualified"] = true.into();
    assert!(Bindings::parse_documents(&[&unknown.to_string()]).is_err());
    Ok(())
}

#[test]
fn compiled_evidence_matches_the_pinned_draccoon_recipe() -> Result<(), String> {
    let evidence = RecipeEvidence::embedded()?;
    let mut bindings = binding()?;
    let recipe = evidence
        .recipes
        .get(&bindings.quests[0].quest)
        .ok_or("canonical chosen recipe")?;
    assert_eq!(
        recipe.digest.as_deref(),
        Some("c6bb54b0c423a165891b0ca32fb305a0963d1a06417249b0b317ec34cf65e708")
    );
    bindings.quests[0].recipe_sha256 = recipe.digest.clone().ok_or("digest")?;
    evidence.verify(&bindings)?;
    Ok(())
}

#[test]
fn explicit_empty_native_bindings_preserve_the_legacy_catalogue() -> Result<(), String> {
    let baseline = crate::quest::loader::lower_embedded_quest_state("native-admission-fixture")
        .map_err(|error| format!("{error:?}"))?;
    let empty = Bindings::parse_documents(&[])?;
    let explicit = crate::quest::loader::load_embedded_quest_state_with_bindings(
        "native-admission-fixture",
        &empty,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(baseline.counts(), explicit.counts());
    let before = baseline.catalogue();
    let after = explicit.catalogue();
    assert_eq!(before.tracks, after.tracks);
    assert_eq!(before.transitions, after.transitions);
    assert_eq!(before.hashes, after.hashes);
    assert_eq!(before.npc_legacy, after.npc_legacy);
    assert_eq!(before.predecessors, after.predecessors);
    assert_eq!(before.authored_bindings, after.authored_bindings);
    assert!(after.native_recipe_evidence.is_some());
    Ok(())
}

#[test]
fn native_recipe_evidence_survives_catalogue_extension() -> Result<(), String> {
    let mut catalogue =
        crate::quest::QuestStateCatalogue::empty("native-admission-fixture").ok_or("revision")?;
    catalogue.native_recipe_evidence = Some(std::sync::Arc::new(RecipeEvidence::embedded()?));
    let extended = catalogue
        .with_quest(
            vec![crate::quest::QuestTrack {
                key: "oteryn:quest-progress/native-admission-extension/phase".into(),
                quest: "oteryn:quest.authored.native_admission_extension".into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![],
        )
        .map_err(|error| format!("{error:?}"))?;
    let rejected = binding()?.attach(&extended);
    assert!(rejected.is_err_and(|error| error.contains("recipe digest differs")));
    Ok(())
}
