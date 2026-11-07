//! D35 regression: run all actual loader extensions immediately before NPC augmentation.
use super::*;
fn immediately_before_source_programme(
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let mut lowered = parse_quest_state(EMBEDDED, content_revision)?;
    lowered.catalogue =
        super::super::world_callbacks::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::monk_shrines::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::threatened_dreams::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::classic_dialogue::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::liquid_black::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::grave_danger::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue = super::super::wrath_sleeping_dragon::extend(lowered.catalogue)
        .map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::grizzly_adams::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::rotten_blood::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::dark_trails::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::super::isle_of_evil::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    Ok(lowered)
}
#[test]
fn srv7_source_acquisition_components_preserve_all_preexisting_quest_definition_pins()
-> Result<(), String> {
    let loaded = immediately_before_source_programme("content-1")
        .map_err(|e| format!("pre-source live loader: {e:?}"))?;
    let program = super::super::npc_cosmetics::source_program::embedded()
        .map_err(|e| format!("source programme: {e:?}"))?;
    let owned: std::collections::BTreeSet<_> = program
        .npcs
        .iter()
        .flat_map(|npc| npc.owned_tracks.iter())
        .collect();
    let missing: Vec<_> = program
        .bindings
        .iter()
        .filter(|b| owned.contains(&b.storage) && loaded.catalogue.track(&b.key).is_none())
        .map(|b| serde_json::json!({"storage":b.storage,"key":b.key,"declared_owner":b.quest}))
        .collect();
    eprintln!(
        "SRV7_PRE_SOURCE_MISSING_TRACKS {}",
        serde_json::to_string(&missing).map_err(|e| e.to_string())?
    );
    let original_hashes = loaded.catalogue.hashes.clone();
    let after = super::super::npc_cosmetics::source_program::extend_catalogue(loaded.catalogue)
        .map_err(|e| format!("source catalogue extension: {e}"))?;
    let changed: Vec<_> = original_hashes
        .into_iter()
        .filter(|(quest, hash)| after.definition_hash(quest) != Some(*hash))
        .map(|(quest, _)| quest)
        .collect();
    assert!(
        changed.is_empty(),
        "D35 existing quest pins changed by acquisition-only track population: {changed:?}"
    );
    Ok(())
}
