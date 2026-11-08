//! Closed final-source acknowledgement tracks; preexisting Quest pins stay intact.
use super::{QuestCatalogueError, QuestStateCatalogue, QuestTrack};
pub const QUEST: &str = "oteryn:quest.npc_cosmetic_final_acquisition";
const TRACKS: [(&str, i64); 4] = [
    (
        "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission",
        7,
    ),
    ("oteryn:quest-progress/navigator", 4),
];
/// This only declares storage shape. Actual chest, quest and room-entry
/// predecessors remain independently owned and are never initialized here.
pub(super) fn extend(
    catalogue: QuestStateCatalogue,
) -> Result<QuestStateCatalogue, QuestCatalogueError> {
    let mut missing = Vec::new();
    for (key, max) in TRACKS {
        if let Some(existing) = catalogue.track(key) {
            let mature_owner = match key {
                "oteryn:quest-progress/quest/u7_24/the_annihilator/reward" => {
                    "oteryn:quest.the_annihilator_quest"
                }
                "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet" => {
                    "oteryn:quest.demon_helmet_quest"
                }
                "oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission" => {
                    "oteryn:quest.the_secret_library_quest"
                }
                "oteryn:quest-progress/navigator" => "oteryn:quest.outfit_and_addon_quests",
                _ => return Err(QuestCatalogueError::DuplicateKey(key.into())),
            };
            // Main's reward callbacks already declare these exact value domains.
            // Reuse their owner without changing its definition or accepting drift.
            if existing.quest != mature_owner
                || (existing.initial, existing.min, existing.max) != (-1, -1, max)
            {
                return Err(QuestCatalogueError::DuplicateKey(key.into()));
            }
        } else {
            missing.push(QuestTrack {
                key: key.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max,
            });
        }
    }
    catalogue.with_quest(missing, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_final_acknowledgements_do_not_change_preexisting_definition_pins()
    -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("source-final-component-test")
            .map_err(|error| format!("actual embedded catalogue: {error:?}"))?;
        let actual = loaded.catalogue();
        let old = QuestStateCatalogue::new(
            actual.content_revision(),
            actual
                .tracks
                .values()
                .filter(|track| track.quest != QUEST)
                .cloned()
                .collect(),
            actual
                .transitions
                .values()
                .filter(|transition| transition.quest != QUEST)
                .cloned()
                .collect(),
        )
        .map_err(|error| format!("original catalogue: {error:?}"))?;
        let hashes = old.hashes.clone();
        let extended = extend(old).map_err(|error| format!("final component: {error:?}"))?;
        for (quest, hash) in hashes {
            assert_eq!(extended.definition_hash(&quest), Some(hash), "D35: {quest}");
        }
        for (key, max) in TRACKS {
            let track = extended
                .track(key)
                .ok_or_else(|| format!("missing final track {key}"))?;
            let expected_owner = match key {
                "oteryn:quest-progress/quest/u7_24/the_annihilator/reward" => {
                    "oteryn:quest.the_annihilator_quest"
                }
                "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet" => {
                    "oteryn:quest.demon_helmet_quest"
                }
                "oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission" => {
                    "oteryn:quest.the_secret_library_quest"
                }
                "oteryn:quest-progress/navigator" => "oteryn:quest.outfit_and_addon_quests",
                _ => QUEST,
            };
            assert_eq!(track.quest, expected_owner);
            assert_eq!((track.initial, track.min, track.max), (-1, -1, max));
        }
        let repeated = extend(extended.clone())
            .map_err(|error| format!("exact mature declarations remain unchanged: {error:?}"))?;
        assert_eq!(repeated.tracks, extended.tracks);
        assert_eq!(repeated.hashes, extended.hashes);
        let component_owned = QuestStateCatalogue::new(
            "duplicate-component-test",
            vec![QuestTrack {
                key: TRACKS[0].0.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 2,
            }],
            Vec::new(),
        )
        .map_err(|error| format!("duplicate component fixture: {error:?}"))?;
        assert!(
            extend(component_owned).is_err(),
            "a duplicate component owner must never rebind a track"
        );
        Ok(())
    }
    #[test]
    fn mature_final_track_reuse_rejects_owner_or_value_domain_drift() -> Result<(), String> {
        for (key, owner, max) in [
            (TRACKS[0].0, "oteryn:quest.the_annihilator_quest", 2),
            (TRACKS[1].0, "oteryn:quest.demon_helmet_quest", 2),
            (TRACKS[2].0, "oteryn:quest.the_secret_library_quest", 7),
            (TRACKS[3].0, "oteryn:quest.outfit_and_addon_quests", 4),
        ] {
            for change in 0..5 {
                let mut track = QuestTrack {
                    key: key.into(),
                    quest: owner.into(),
                    initial: -1,
                    min: -1,
                    max,
                };
                match change {
                    0 => {}
                    1 => track.quest = QUEST.into(),
                    2 => track.initial = 0,
                    3 => track.min = -2,
                    _ => track.max = max + 1,
                }
                let catalogue =
                    QuestStateCatalogue::new("compatibility-test", vec![track.clone()], Vec::new())
                        .map_err(|error| format!("fixture: {error:?}"))?;
                let old_hash = catalogue.definition_hash(owner);
                let result = extend(catalogue);
                if change == 0 {
                    let actual =
                        result.map_err(|error| format!("exact mature declaration: {error:?}"))?;
                    assert_eq!(actual.track(key), Some(&track));
                    assert_eq!(actual.definition_hash(owner), old_hash);
                } else {
                    assert!(
                        matches!(result, Err(QuestCatalogueError::DuplicateKey(ref duplicate)) if duplicate == key)
                    );
                }
            }
        }
        Ok(())
    }
}
