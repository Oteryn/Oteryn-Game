//! Finite journal composition for the questpc-owned Kingsday recipe.
use super::*;

pub const QUEST: &str = "oteryn:quest.authored.kingsday_mini_world_change";
pub const TRACK: &str = "oteryn:quest-progress/questpc/kingsday/progress";

pub fn extend(
    base: &QuestStateCatalogue,
    journal: &QuestLogContent,
) -> Result<(QuestStateCatalogue, QuestLogContent), String> {
    let bindings = crate::quest::authored_bindings::Bindings::parse(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/questpc-authored-bindings.json"
    )))?;
    let state = bindings.attach(base)?;
    let mut quests = journal.active.quests.clone();
    quests.push(QuestLogQuest {
        key: QUEST.into(), kind: QuestLogKind::Storyline, shown_in_quest_log: true,
        start: Some((TRACK.into(), 1)), hide_when_completed: false,
        missions: vec![QuestLogMission {
            name: "Kingsday Mini World Change".into(), track: TRACK.into(),
            start_value: 1, end_value: 3,
            journal: QuestJournal::PerStage([
                (1, "I visited the celebration outside Thais castle. I should greet King Tibianus respectfully."),
                (2, "I greeted King Tibianus. I can confirm Kingsday to record my celebration."),
                (3, "I celebrated Kingsday and became a Loyal Subject."),
            ].into_iter().map(|(stage, text)| (stage, JournalText::Text(text.into()))).collect()),
        }],
    });
    let active = QuestLogCatalogue::new(&state, quests).map_err(|e| format!("{e:?}"))?;
    Ok((
        state,
        QuestLogContent::new(active, journal.pinned.values().cloned().collect()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kingsday_boot_composition_retains_previous_catalogues() -> Result<(), String> {
        let lowered = crate::quest::loader::load_embedded_quest_state("kingsday-test")
            .map_err(|e| format!("{e:?}"))?;
        let loaded = loader::load_embedded_quest_log(&lowered).map_err(|e| format!("{e:?}"))?;
        let base =
            crate::quest::task_board::extend(lowered.catalogue()).map_err(|e| format!("{e:?}"))?;
        let (state, journal) = extend(&base, &loaded.content)?;
        assert_eq!(
            state.track(TRACK).map(|t| (t.initial, t.min, t.max)),
            Some((0, 0, 3))
        );
        for quest in loaded.content.active().quests() {
            assert_eq!(
                state.definition_hash(&quest.key),
                base.definition_hash(&quest.key)
            );
            assert!(journal.active().quests().contains(quest));
        }
        for previous in loaded.content.catalogues().skip(1) {
            assert!(journal.catalogues().any(|retained| retained == previous));
        }
        assert!(journal.active().quests().iter().any(|q| q.key == QUEST));
        Ok(())
    }
}
