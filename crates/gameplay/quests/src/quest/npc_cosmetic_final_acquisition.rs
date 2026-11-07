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
    catalogue.with_quest(
        TRACKS
            .iter()
            .map(|(key, max)| QuestTrack {
                key: (*key).into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: *max,
            })
            .collect(),
        Vec::new(),
    )
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
            assert_eq!(track.quest, QUEST);
            assert_eq!((track.initial, track.min, track.max), (-1, -1, max));
        }
        assert!(
            extend(extended).is_err(),
            "a duplicate owner must never rebind a track"
        );
        Ok(())
    }
}
