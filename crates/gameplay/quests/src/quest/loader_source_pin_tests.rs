//! D35 regression: run all actual loader extensions immediately before NPC augmentation.
use super::*;
fn immediately_before_source_programme(
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let mut lowered = load_embedded_quest_state(content_revision)?;
    // Remove only actual acquisition-component declarations. Main's mature
    // reward declarations and every existing definition pin remain admitted.
    lowered.catalogue = QuestStateCatalogue::new(
        content_revision,
        lowered
            .catalogue
            .tracks
            .values()
            .filter(|track| track.quest != "oteryn:quest.npc_cosmetic_acquisition")
            .cloned()
            .collect(),
        lowered.catalogue.transitions.values().cloned().collect(),
    )
    .map_err(|error| QuestLoadError::Json(format!("pre-source component removal: {error:?}")))?;
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
    let mature = [
        (
            "oteryn:quest-progress/dream_outfit",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/outfit_quest/ref",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            0,
            2147483647,
        ),
        (
            "oteryn:quest-progress/quest/u11_80/battle_mage_outfits/addon1",
            "oteryn:quest.battle_mage_outfits_quest",
            -1,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/quest/u11_80/battle_mage_outfits/addon2",
            "oteryn:quest.battle_mage_outfits_quest",
            -1,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/quest/u7_8/beggar_outfits/beggar_first_addon_door",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/quest/u7_8/hunter_outfits/hunter/addon_glove",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/quest/u7_8/knight_outfits/addon_helmet",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            7,
        ),
        (
            "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_hat_cloak",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            11,
        ),
        (
            "oteryn:quest-progress/wayfarer_outfit",
            "oteryn:quest.wayfarer_outfits_quest",
            -1,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/dream_outfit_1",
            "oteryn:quest.outfit_and_addon_quests",
            0,
            -1,
            1,
        ),
        (
            "oteryn:quest-progress/wayfarer_outfit_1",
            "oteryn:quest.wayfarer_outfits_quest",
            -1,
            -1,
            1,
        ),
    ];
    assert_eq!(mature.len(), 11);
    assert_eq!(missing.len(), 1);
    let component_keys: std::collections::BTreeSet<_> = missing
        .iter()
        .map(|row| row["key"].as_str().ok_or("missing component key"))
        .collect::<Result<_, _>>()?;
    assert_eq!(
        component_keys,
        std::collections::BTreeSet::from([
            "oteryn:quest-progress/npc-cosmetic/rock_steady/storage_72326",
        ])
    );
    for (key, quest, initial, min, max) in mature {
        let track = loaded
            .catalogue
            .track(key)
            .ok_or_else(|| format!("missing mature declaration {key}"))?;
        assert_eq!(
            (&*track.quest, track.initial, track.min, track.max),
            (quest, initial, min, max)
        );
        // The existing strict drift and existing-definition guards must remain active.
        for remove in [false, true] {
            let tracks = loaded
                .catalogue
                .tracks
                .values()
                .filter_map(|track| {
                    if track.key != key {
                        return Some(track.clone());
                    }
                    if remove {
                        return None;
                    }
                    let mut wrong_owner = track.clone();
                    wrong_owner.quest = "oteryn:quest.npc_cosmetic_acquisition".into();
                    Some(wrong_owner)
                })
                .collect();
            // Removing a referenced track can itself be rejected by the stricter
            // catalogue constructor; otherwise source augmentation must refuse it.
            if let Ok(incompatible) = QuestStateCatalogue::new(
                "content-1",
                tracks,
                loaded.catalogue.transitions.values().cloned().collect(),
            ) {
                assert!(
                    super::super::npc_cosmetics::source_program::extend_catalogue(incompatible)
                        .is_err(),
                    "mature declaration drift {key}, remove={remove}"
                );
            }
        }
    }
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
