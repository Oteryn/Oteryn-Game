//! Stampede's personal five-kill goal. Source: Crystal 201189f
//! data/scripts/creaturescripts/monster/terrified_elephant_death.lua:1-29.
//! Daily placement already belongs to the source-bound Stampede raid.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:quest.stampede_mini_world_change";
pub const CREATURE: &str = "oteryn:creature.terrified_elephant";
pub const TRACK: &str = "oteryn:quest-progress/stampede/kills";
pub const KILL: &str = "oteryn:quest-transition/stampede/kill";
pub const THRESHOLD: i64 = 5;
pub const ACHIEVEMENT: &str = "oteryn:achievement/trail_of_the_ape_god";

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.into_values().collect();
    tracks.push(QuestTrack {
        key: TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: THRESHOLD,
    });
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    transitions.push(QuestTransition {
        world_change: None,
        key: KILL.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: TRACK.into(),
            from: QuestComparison::Between(0, THRESHOLD - 1),
            effect: QuestEffectKind::Add(1),
        }],
        completes: false,
        experience: None,
    });
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::QuestRefusal;
    use std::collections::BTreeMap;

    #[test]
    fn stampede_embedded_counter_counts_five_native_kills_and_then_refuses() -> Result<(), String> {
        let lowered = crate::quest::loader::load_embedded_quest_state("stampede-unit")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let transition = catalogue
            .transition(KILL)
            .ok_or("Stampede counter not loaded")?;
        let mut stored = BTreeMap::new();
        for count in 1..=5 {
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("{error:?}"))?;
            let [change] = changes.as_slice() else {
                return Err("one source storage write".into());
            };
            assert_eq!((change.before, change.after), (count - 1, count));
            stored.insert(change.track.clone(), change.after);
        }
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }
}
