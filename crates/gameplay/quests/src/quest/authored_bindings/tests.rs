use super::*;

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";

fn fixture() -> Result<Bindings, String> {
    serde_json::from_value(serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":"1".repeat(64),"sources":["engine fixture"],
            "tracks":[{"key":"oteryn:quest-progress/authored-engine/phase","initial":0,"minimum":0,"maximum":1}],
            "stages":[{"key":"delivery","completes":true,"trigger":{
                "kind":"npc_dialogue","callback":{
                    "key":"oteryn:quest-reward/authored-engine-unit","quest":QUEST,"npc":"oteryn:npc.cledwyn",
                    "keywords":["enginebinding"],"topic":0,"next_topic":0,
                    "storage":[{"track":"oteryn:quest-progress/authored-engine/phase","minimum":0,"maximum":0,"set":1}],
                    "take":[],"give":{"item":"oteryn:item.tibia.i3031","count":1,"contents":[]},
                    "outfit":null,"achievement":null,"reply":["Bound stage."],"sources":["engine fixture"]
                }
            }}]
        }]
    })).map_err(|error| error.to_string())
}

#[test]
fn closed_binding_document_rejects_unknown_fields_and_foreign_storage() -> Result<(), String> {
    let binding = fixture()?;
    let bytes = serde_json::to_vec(&binding).map_err(|error| error.to_string())?;
    assert_eq!(Bindings::parse(&bytes)?, binding);
    let mut unknown = serde_json::to_value(&binding).map_err(|error| error.to_string())?;
    unknown["quests"][0]["stages"][0]["trigger"]["guessed_target"] = true.into();
    assert!(
        Bindings::parse(&serde_json::to_vec(&unknown).map_err(|error| error.to_string())?).is_err()
    );
    let mut foreign = binding;
    if let Trigger::NpcDialogue { callback, .. } = &mut foreign.quests[0].stages[0].trigger {
        callback.storage[0].track = "oteryn:quest-progress/foreign".into();
    }
    assert!(foreign.validate().is_err());
    Ok(())
}

#[test]
fn native_binding_identity_and_recipe_digest_are_part_of_definition_hash() -> Result<(), String> {
    let base = QuestStateCatalogue::empty("engine-fixture-r1").ok_or("base")?;
    let binding = fixture()?;
    let catalogue = binding.attach(&base)?;
    let before = catalogue.definition_hash(QUEST).ok_or("hash")?;
    let mut changed = binding.clone();
    if let Trigger::NpcDialogue { callback, .. } = &mut changed.quests[0].stages[0].trigger {
        callback.npc = "oteryn:npc.benjamin".into();
    }
    assert_ne!(changed.attach(&base)?.definition_hash(QUEST), Some(before));
    changed = binding;
    changed.quests[0].recipe_sha256 = "2".repeat(64);
    assert_ne!(changed.attach(&base)?.definition_hash(QUEST), Some(before));
    Ok(())
}

#[test]
fn extending_catalogue_preserves_bound_identity_and_runtime_callbacks() -> Result<(), String> {
    let base = QuestStateCatalogue::empty("engine-fixture-r1").ok_or("base")?;
    let binding = fixture()?;
    let catalogue = binding.attach(&base)?;
    let extended = catalogue
        .with_quest(
            vec![QuestTrack {
                key: "oteryn:quest-progress/another-engine-fixture/phase".into(),
                quest: "oteryn:quest.authored.rascoohan_outfits_quest".into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![],
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        extended.definition_hash(QUEST),
        catalogue.definition_hash(QUEST)
    );
    assert_eq!(extended.authored_bindings(), catalogue.authored_bindings());
    assert!(
        super::super::reward_callbacks::load_for(&extended)?
            .iter()
            .any(|callback| callback.key == "oteryn:quest-reward/authored-engine-unit")
    );
    Ok(())
}

#[test]
fn base_callback_collisions_are_refused_before_catalogue_admission() -> Result<(), String> {
    let base = QuestStateCatalogue::empty("engine-fixture-r1").ok_or("base")?;
    let existing = super::super::reward_callbacks::load()?
        .into_iter()
        .next()
        .ok_or("existing callback")?;
    let mut binding = fixture()?;
    if let Trigger::NpcDialogue { callback, .. } = &mut binding.quests[0].stages[0].trigger {
        callback.key = existing.key;
    }
    assert!(binding.attach(&base).is_err());
    Ok(())
}

#[test]
fn native_program_hashes_and_predecessors_survive_authored_attachment() -> Result<(), String> {
    let other = "oteryn:quest.authored.rascoohan_outfits_quest";
    let track = "oteryn:quest-progress/another-engine-fixture/phase";
    let mut base = QuestStateCatalogue::new(
        "engine-fixture-r1",
        vec![QuestTrack {
            key: track.into(),
            quest: other.into(),
            initial: 0,
            min: 0,
            max: 1,
        }],
        vec![],
    )
    .map_err(|error| format!("{error:?}"))?;
    let defaults = std::collections::BTreeMap::from([(track.into(), 0)]);
    base.hashes.insert(other.into(), [7; 32]);
    base.npc_legacy
        .insert(other.into(), ([1; 32], defaults.clone()));
    base.predecessors.insert(
        other.into(),
        vec![super::super::NpcDefinitionPredecessor {
            hash: [2; 32],
            defaults,
        }],
    );
    let attached = fixture()?.attach(&base)?;
    assert_eq!(
        attached.definition_hash(other),
        Some([7; 32]),
        "existing native-program binding remains part of its definition"
    );
    assert_eq!(attached.npc_legacy, base.npc_legacy);
    assert_eq!(attached.predecessors, base.predecessors);
    let extended = base
        .with_quest(
            vec![QuestTrack {
                key: "oteryn:quest-progress/third-engine-fixture/phase".into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![],
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(extended.npc_legacy, base.npc_legacy);
    assert_eq!(extended.predecessors, base.predecessors);
    Ok(())
}

#[test]
fn changed_authored_quest_does_not_inherit_an_npc_only_revision_allowance() -> Result<(), String> {
    let mut base = QuestStateCatalogue::new(
        "engine-fixture-r1",
        vec![QuestTrack {
            key: "oteryn:quest-progress/existing-engine-fixture/phase".into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 1,
        }],
        vec![],
    )
    .map_err(|error| format!("{error:?}"))?;
    base.hashes.insert(QUEST.into(), [7; 32]);
    base.npc_legacy
        .insert(QUEST.into(), ([1; 32], std::collections::BTreeMap::new()));
    base.predecessors.insert(
        QUEST.into(),
        vec![super::super::NpcDefinitionPredecessor {
            hash: [2; 32],
            defaults: std::collections::BTreeMap::new(),
        }],
    );
    let attached = fixture()?.attach(&base)?;
    assert!(!attached.npc_legacy.contains_key(QUEST));
    assert!(!attached.predecessors.contains_key(QUEST));
    let mut different_native_program = base;
    different_native_program
        .hashes
        .insert(QUEST.into(), [8; 32]);
    assert_ne!(
        attached.definition_hash(QUEST),
        fixture()?
            .attach(&different_native_program)?
            .definition_hash(QUEST),
        "the prior native program's binding remains part of the new authored definition"
    );
    Ok(())
}

#[test]
fn attached_stage_completion_matches_the_durable_callback_plan() -> Result<(), String> {
    let base = QuestStateCatalogue::empty("engine-fixture-r1").ok_or("base")?;
    let bindings = fixture()?;
    let raw = bindings.reward_callbacks().next().ok_or("raw callback")?;
    assert!(!raw.completes, "legacy authored callback omits completion");
    let catalogue = bindings.attach(&base)?;
    let attached = catalogue.authored_bindings().ok_or("attached bindings")?;
    let callback = attached
        .reward_callbacks()
        .next()
        .ok_or("attached callback")?;
    assert!(callback.completes);
    assert_eq!(
        catalogue.transition(&callback.key),
        Some(&super::super::reward_callbacks::primary_transition(
            callback
        )?)
    );
    assert_eq!(
        attached.attach(&base)?.definition_hash(QUEST),
        catalogue.definition_hash(QUEST)
    );
    Ok(())
}
