//! Crystal 9f5a72c npc/santiago.lua:67-71: first greeting is one atomic write.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack};

pub const SANTIAGO: &str = "oteryn:npc.santiago";
pub const GREET: &str =
    "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_npc_greet_storage";
pub const START: &str =
    "oteryn:quest-transition/quest/u8_2/the_beginning_quest/santiago_quest_log/npc_1";

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks;
    let mut transitions = catalogue.transitions;
    let start = transitions
        .get_mut(START)
        .ok_or("missing Santiago source start")?;
    let track = QuestTrack {
        key: GREET.into(),
        quest: start.quest.clone(),
        initial: -1,
        min: -1,
        max: 13,
    };
    if tracks.insert(GREET.into(), track).is_some() {
        return Err("duplicate Santiago greeting track".into());
    }
    start.effects.push(QuestEffect {
        track: GREET.into(),
        from: QuestComparison::Le(0),
        effect: QuestEffectKind::Set(1),
    });
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn santiago_first_greeting_has_one_atomic_guarded_start() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("beginning-greet")
            .map_err(|e| format!("{e:?}"))?;
        let book = loaded.catalogue();
        let start = book.transition(START).ok_or("start")?;
        let changes = book
            .evaluate(start, &BTreeMap::new(), 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|change| change.after == 1));
        assert_eq!(
            book.evaluate(start, &BTreeMap::from([(GREET.into(), 1)]), 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        Ok(())
    }
}
