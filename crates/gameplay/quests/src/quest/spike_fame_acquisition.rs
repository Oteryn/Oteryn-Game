//! Acquisition-only Fame component; existing Spike task hashes are unchanged.
use super::{QuestCatalogueError, QuestStateCatalogue, QuestTrack};
pub const QUEST: &str = "oteryn:quest.spike_fame_acquisition";
pub const TRACK: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_fame_points";

pub(super) fn extend(
    catalogue: QuestStateCatalogue,
) -> Result<QuestStateCatalogue, QuestCatalogueError> {
    catalogue.with_quest(
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: i64::from(i32::MAX),
        }],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_spike_fame_component_preserves_all_existing_definition_pins() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("actual embedded quests: {error:?}"))?;
        let actual = loaded.catalogue();
        // Remove only this component if the production loader already appended it.
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
        .map_err(|error| format!("existing catalogue: {error:?}"))?;
        let old_hashes = old.hashes.clone();
        let extended = extend(old).map_err(|error| format!("Fame extension: {error:?}"))?;
        for (quest, hash) in old_hashes {
            assert_eq!(extended.definition_hash(&quest), Some(hash), "D35: {quest}");
        }
        for key in [
            "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_kill_main",
            "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_kill_daily",
        ] {
            assert_eq!(
                extended.track(key).map(|track| track.quest.as_str()),
                Some("oteryn:quest.spike_task")
            );
        }
        let fame = extended.track(TRACK).ok_or("Fame track")?;
        assert_eq!(
            (fame.quest.as_str(), fame.initial, fame.min, fame.max),
            (QUEST, -1, -1, i64::from(i32::MAX))
        );
        assert!(
            extend(extended).is_err(),
            "a second owner cannot rebind this track"
        );
        Ok(())
    }
}
