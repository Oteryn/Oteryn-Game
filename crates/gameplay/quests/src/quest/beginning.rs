//! Crystal 9f5a72c npc/santiago.lua:67-71: first greeting is one atomic write.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const SANTIAGO: &str = "oteryn:npc.santiago";
pub const GREET: &str =
    "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_npc_greet_storage";
/// Crystal quest_system1.lua:166-171: the coat chest advances greeting only.
pub const COAT_CHEST: &str = "oteryn:quest-transition/kv/beginning_coat/chest";
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
    let chest = QuestTransition {
        key: COAT_CHEST.into(),
        quest: start.quest.clone(),
        effects: vec![QuestEffect {
            track: GREET.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(3),
        }],
        completes: false,
        experience: None,
    };
    if transitions.insert(COAT_CHEST.into(), chest).is_some() {
        return Err("duplicate Beginning coat chest transition".into());
    }
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
    fn source_coat_chest_changes_greeting_only() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("beginning-coat")
            .map_err(|error| format!("{error:?}"))?;
        let book = loaded.catalogue();
        let transition = book.transition(COAT_CHEST).ok_or("chest transition")?;
        assert!(!transition.completes);
        assert_eq!(transition.experience, None);
        let changes = book
            .evaluate(transition, &BTreeMap::new(), 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].track, GREET);
        assert_eq!(changes[0].after, 3);
        Ok(())
    }

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
