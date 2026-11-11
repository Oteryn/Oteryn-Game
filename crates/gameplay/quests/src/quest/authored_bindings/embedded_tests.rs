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
        let super::super::Trigger::NpcDialogue { callback, .. } = &mut stage.trigger else {
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

#[test]
fn talk_stage_callback_group_is_closed_and_part_of_the_catalogue() -> Result<(), String> {
    let original: Value = serde_json::from_str(include_str!("fixtures/talk.json"))
        .map_err(|error| error.to_string())?;
    let mut document = original.clone();
    let mut alternative = document["quests"][0]["stages"][0]["trigger"]["callback"].clone();
    alternative["key"] = "oteryn:quest-reward/authored-talk-alternative".into();
    alternative["npc"] = "oteryn:npc.palimuth".into();
    document["quests"][0]["stages"][0]["trigger"]["alternatives"] =
        serde_json::json!([alternative]);
    let bindings = Bindings::parse(document.to_string().as_bytes())?;
    assert_eq!(bindings.reward_callbacks().count(), 2);
    let base =
        crate::quest::QuestStateCatalogue::empty("talk-group-test").ok_or("content revision")?;
    let attached = bindings.attach(&base)?;
    assert!(
        attached
            .transition("oteryn:quest-reward/authored-talk-alternative")
            .is_some()
    );
    for callback in bindings.reward_callbacks() {
        let transition = attached
            .transition(&callback.key)
            .ok_or("registered TALK operation")?;
        crate::quest::admission_binding::validate_for_catalogue(&attached, transition, false)?;
        assert!(
            crate::quest::admission_binding::validate_for_catalogue(&attached, transition, true)
                .is_err()
        );
        let mut forged = transition.clone();
        forged.effects[0].effect = crate::quest::QuestEffectKind::Set(0);
        assert!(
            crate::quest::admission_binding::validate_for_catalogue(&attached, &forged, false)
                .is_err()
        );
        forged.key.push_str("-alias");
        assert!(
            crate::quest::admission_binding::validate_for_catalogue(&attached, &forged, false)
                .is_err()
        );
    }
    let before = Bindings::parse(original.to_string().as_bytes())?.attach(&base)?;
    assert_ne!(
        attached.definition_hash(&bindings.quests[0].quest),
        before.definition_hash(&bindings.quests[0].quest)
    );
    let mut duplicate = document.clone();
    duplicate["quests"][0]["stages"][0]["trigger"]["alternatives"][0]["key"] =
        document["quests"][0]["stages"][0]["trigger"]["callback"]["key"].clone();
    assert!(Bindings::parse(duplicate.to_string().as_bytes()).is_err());
    let mut foreign = document;
    foreign["quests"][0]["stages"][0]["trigger"]["alternatives"][0]["storage"][0]["track"] =
        "oteryn:quest-progress/foreign".into();
    assert!(Bindings::parse(foreign.to_string().as_bytes()).is_err());
    Ok(())
}

#[test]
fn imported_talk_reuses_owned_bounded_state_operations() -> Result<(), String> {
    let mut document: Value = serde_json::from_str(include_str!("fixtures/talk.json"))
        .map_err(|error| error.to_string())?;
    let operation = serde_json::json!({
        "track":"oteryn:quest-progress/authored-kills/count",
        "from":{"kind":"between","minimum":0,"maximum":2},
        "effect":{"kind":"add","value":1}
    });
    document["quests"][0]["stages"][0]["trigger"]["callback"]["state_ops"] =
        serde_json::json!([operation]);
    let bindings = Bindings::parse(document.to_string().as_bytes())?;
    let base =
        crate::quest::QuestStateCatalogue::empty("talk-state-ops").ok_or("content revision")?;
    let catalogue = bindings.attach(&base)?;
    let callback = bindings.reward_callbacks().next().ok_or("callback")?;
    let transition = catalogue.transition(&callback.key).ok_or("transition")?;
    let tracks = std::collections::BTreeMap::from([
        ("oteryn:quest-progress/authored-kills/phase".into(), 0),
        ("oteryn:quest-progress/authored-kills/count".into(), 2),
    ]);
    let changes = catalogue
        .evaluate(transition, &tracks, 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        changes
            .iter()
            .find(|change| change.track.ends_with("/count"))
            .ok_or("counter")?
            .after,
        3
    );
    let mut foreign = document.clone();
    foreign["quests"][0]["stages"][0]["trigger"]["callback"]["state_ops"][0]["track"] =
        "oteryn:quest-progress/foreign".into();
    assert!(Bindings::parse(foreign.to_string().as_bytes()).is_err());
    let mut overflow = document;
    overflow["quests"][0]["stages"][0]["trigger"]["callback"]["state_ops"][0]["from"]["maximum"] =
        3.into();
    assert!(Bindings::parse(overflow.to_string().as_bytes()).is_err());
    Ok(())
}

#[test]
fn native_partial_claim_hold_allows_only_admission_free_nonreward_progress() -> Result<(), String> {
    for document in [
        super::super::terrain_use_tests::document(),
        serde_json::from_slice(include_bytes!("fixtures/door_use.json"))
            .map_err(|error| error.to_string())?,
    ] {
        let mut bindings =
            Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
        let index = serde_json::json!({"schema":"OTERYN_FAMILY_INDEX/v1","family":"Quest","shards":["fixture-shard"],"record_count":1});
        let recipe = serde_json::json!({"requirements":{"min_level":0,"premium":false,"prerequisites":[]},"stages":[{"key":"s1","kind":"use","count":1}]});
        let identity = serde_json::json!({"key":bindings.quests[0].quest,"revision":"authored-r1"});
        let mut shard = serde_json::json!({"schema":"OTERYN_QUEST_TREE_SHARD/v1","family":"Quest","records":[{"definition":{
            "identity":identity,"missing_data":[{"code":"claim_native_lowering_missing"}],
            "oteryn_recipe":{"profile":"chosen_source_completion_v1","chosen_data_complete":true,"payload":{"identity":identity,"recipe":recipe}}
        }}]});
        let repin = |bindings: &mut Bindings, shard: &Value| -> Result<(), String> {
            bindings.quests[0].recipe_sha256 = Sha256::digest(
                serde_json::to_vec(
                    &shard["records"][0]["definition"]["oteryn_recipe"]["payload"]["recipe"],
                )
                .map_err(|e| e.to_string())?,
            )
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
            Ok(())
        };
        repin(&mut bindings, &shard)?;
        evidence(&index, &shard)?.verify(&bindings)?;
        let mut completion = bindings.clone();
        completion.quests[0].stages[0].completes = true;
        assert!(evidence(&index, &shard)?.verify(&completion).is_err());
        for (field, value) in [
            ("min_level", serde_json::json!(50)),
            ("premium", serde_json::json!(true)),
            ("prerequisites", serde_json::json!(["Other quest"])),
        ] {
            let mut denied = shard.clone();
            denied["records"][0]["definition"]["oteryn_recipe"]["payload"]["recipe"]["requirements"]
                [field] = value;
            let mut bound = bindings.clone();
            repin(&mut bound, &denied)?;
            assert!(
                evidence(&index, &denied)?.verify(&bound).is_err(),
                "actor admission bypass: {field}"
            );
        }
        shard["records"][0]["definition"]["missing_data"] = serde_json::json!([{"code":"claim_native_lowering_missing"},{"code":"claim_item_semantics_missing"}]);
        assert!(evidence(&index, &shard)?.verify(&bindings).is_err());
    }
    Ok(())
}

#[test]
fn typed_premium_admission_must_match_the_canonical_recipe() -> Result<(), String> {
    let mut document = super::super::terrain_use_tests::document();
    document["quests"][0]["admission"] = serde_json::json!({"minimum_level":0,"premium":true});
    let mut bindings = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    let recipe = serde_json::json!({"requirements":{"min_level":0,"premium":true,"prerequisites":[]},"stages":[{"key":"s1","kind":"use","count":1}]});
    bindings.quests[0].recipe_sha256 =
        Sha256::digest(serde_json::to_vec(&recipe).map_err(|e| e.to_string())?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    let identity = serde_json::json!({"key":bindings.quests[0].quest,"revision":"authored-r1"});
    let index = serde_json::json!({"schema":"OTERYN_FAMILY_INDEX/v1","family":"Quest","shards":["fixture-shard"],"record_count":1});
    let shard = serde_json::json!({"schema":"OTERYN_QUEST_TREE_SHARD/v1","family":"Quest","records":[{"definition":{
        "identity":identity,"missing_data":[{"code":"claim_native_lowering_missing"}],
        "oteryn_recipe":{"profile":"chosen_source_completion_v1","chosen_data_complete":true,"payload":{"identity":identity,"recipe":recipe}}
    }}]});
    evidence(&index, &shard)?.verify(&bindings)?;
    bindings.quests[0].admission = None;
    assert!(
        evidence(&index, &shard)?.verify(&bindings).is_err(),
        "premium may not be implicit"
    );
    document["quests"][0]["admission"]["premium"] = false.into();
    let mut wrong = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    wrong.quests[0].recipe_sha256 = bindings.quests[0].recipe_sha256.clone();
    assert!(evidence(&index, &shard)?.verify(&wrong).is_err());
    Ok(())
}

#[test]
fn typed_prerequisite_admission_must_match_the_chosen_requirement_names() -> Result<(), String> {
    let mut document = super::super::terrain_use_tests::document();
    document["quests"][0]["admission"] = serde_json::json!({"minimum_level":0,"prerequisites":[{"requirement":"Other quest","quest":"oteryn:quest.m01c_dependency"}]});
    let mut bindings = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    let recipe = serde_json::json!({"requirements":{"min_level":0,"premium":false,"prerequisites":["Other quest"]},"stages":[{"key":"s1","kind":"use","count":1}]});
    bindings.quests[0].recipe_sha256 =
        Sha256::digest(serde_json::to_vec(&recipe).map_err(|e| e.to_string())?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    let identity = serde_json::json!({"key":bindings.quests[0].quest,"revision":"authored-r1"});
    let index = serde_json::json!({"schema":"OTERYN_FAMILY_INDEX/v1","family":"Quest","shards":["fixture-shard"],"record_count":1});
    let shard = serde_json::json!({"schema":"OTERYN_QUEST_TREE_SHARD/v1","family":"Quest","records":[{"definition":{
        "identity":identity,"missing_data":[{"code":"claim_native_lowering_missing"}],
        "oteryn_recipe":{"profile":"chosen_source_completion_v1","chosen_data_complete":true,"payload":{"identity":identity,"recipe":recipe}}
    }}]});
    evidence(&index, &shard)?.verify(&bindings)?;
    bindings.quests[0].admission = None;
    assert!(evidence(&index, &shard)?.verify(&bindings).is_err());
    document["quests"][0]["admission"]["prerequisites"][0]["requirement"] =
        "Different quest".into();
    let mut wrong = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    wrong.quests[0].recipe_sha256 = bindings.quests[0].recipe_sha256.clone();
    assert!(evidence(&index, &shard)?.verify(&wrong).is_err());
    Ok(())
}

#[test]
fn chosen_level_resolves_only_the_original_unknown_level() -> Result<(), String> {
    let mut doc: Value = serde_json::from_slice(include_bytes!("fixtures/task_board_use.json"))
        .map_err(|e| e.to_string())?;
    doc["quests"][0]["admission"] = serde_json::json!({"minimum_level":100});
    let mut bindings = Bindings::parse(doc.to_string().as_bytes())?;
    let recipe = serde_json::json!({"requirements":{"basis":"CHOSEN_OTERYN_APPROXIMATION","min_level":100,"premium":false,"prerequisites":[]},"stages":[{"key":"s1","kind":"use","count":1}]});
    bindings.quests[0].recipe_sha256 =
        Sha256::digest(serde_json::to_vec(&recipe).map_err(|e| e.to_string())?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    let index = serde_json::json!({"schema":"OTERYN_FAMILY_INDEX/v1","family":"Quest","shards":["fixture-shard"],"record_count":1});
    let shard = serde_json::json!({"schema":"OTERYN_QUEST_TREE_SHARD/v1","family":"Quest","records":[{"definition":{
        "identity":{"key":bindings.quests[0].quest},
        "missing_data":[{"code":"requirement_unknown","field":"min_level"},{"code":"quest_native_lowering_missing"}],
        "oteryn_recipe":{"profile":"chosen_source_completion_v1","chosen_data_complete":true,
            "payload":{"identity":{"key":bindings.quests[0].quest},"recipe":recipe}}
    }}]});
    evidence(&index, &shard)?.verify(&bindings)?;
    for field in ["premium", "vocation", "unresolved_items"] {
        let mut held = shard.clone();
        held["records"][0]["definition"]["missing_data"][0]["field"] = field.into();
        assert!(
            evidence(&index, &held)?.verify(&bindings).is_err(),
            "unresolved {field} remains blocked"
        );
    }
    let mut held = shard;
    let definition = &mut held["records"][0]["definition"];
    definition["definition_profile"] = "oteryn_authored_v1".into();
    definition["data_complete"] = true.into();
    definition["recipe"] = recipe;
    assert!(
        evidence(&index, &held)?.verify(&bindings).is_err(),
        "native authored gaps are never donor gaps"
    );
    Ok(())
}

#[test]
fn qm55_save_scope_requires_daily_canonical_recipe_even_with_a_matching_digest()
-> Result<(), String> {
    let (bindings, index, shard) = canonical_inputs()?;
    let original = serde_json::to_value(bindings).map_err(|e| e.to_string())?;
    for repeat in ["once", "daily"] {
        let mut source = shard.clone();
        source["records"][0]["definition"]["recipe"]["repeat"] = serde_json::json!({"kind":repeat});
        let mut value = original.clone();
        let quest = &mut value["quests"][0];
        quest["recipe_sha256"] = Sha256::digest(
            serde_json::to_vec(&source["records"][0]["definition"]["recipe"])
                .map_err(|e| e.to_string())?,
        )
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
        let mut restart = quest["stages"][0]["trigger"]["callback"].clone();
        restart["key"] = crate::quest::shared_world_progress::save_event_restart_key(
            quest["quest"].as_str().ok_or("quest")?,
        )
        .into();
        restart["storage"]=serde_json::Value::Array(quest["tracks"].as_array().ok_or("tracks")?.iter().map(|t|serde_json::json!({"track":t["key"],"minimum":t["minimum"],"maximum":t["maximum"],"set":t["initial"]})).collect());
        quest["occurrence"] =
            serde_json::json!({"kind":"server_save","world_change":"grimvale","restart":restart});
        let bindings = Bindings::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?;
        assert_eq!(
            evidence(&index, &source)?.verify(&bindings).is_ok(),
            repeat == "daily"
        );
    }
    Ok(())
}

// Preserve main's original recipe, population and catalogue-extension witnesses.
mod main_native_contract {
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

}
