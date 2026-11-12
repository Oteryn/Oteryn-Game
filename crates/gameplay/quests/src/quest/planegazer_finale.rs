//! Opticording Sphere's post-battle title dialogue.
//! Retained Fandom page65057 revid1177498 lines576-579, captured structured final_rewards:
//! wiki-source-all373/source-specs-373.json entry195/source_fact_supplements/1/fact125.
//! Earlier fight progression remains owned by the quest producer.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
pub const NPC: &str = "oteryn:npc.planestrider_npc";
pub const QUEST: &str = "oteryn:quest.opticording_sphere_quest";
pub const READY: &str = "oteryn:quest-progress/opticording_sphere/planestrider_defeated";
pub const CLAIMED: &str = "oteryn:quest-progress/opticording_sphere/final_report";
pub const COMPLETE: &str = "oteryn:quest-transition/opticording-sphere/planegazer-final-report";

/// The seven exact source words. Incorrect input cannot advance; greeting restarts the dialogue.
pub fn respond(topic: u16, message: &str) -> Option<(u16, bool)> {
    let word = message.to_ascii_lowercase();
    if word == "hi" {
        return Some((1, false));
    }
    let expected = match topic {
        1 => "no",
        2 => "yes",
        3 => "no",
        4..=6 => "yes",
        _ => return None,
    };
    (word == expected).then_some((if topic == 6 { 0 } else { topic + 1 }, topic == 6))
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks;
    for key in [READY, CLAIMED] {
        let track = QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 1,
        };
        if tracks.get(key).is_some_and(|existing| *existing != track) {
            return Err(format!("Planegazer source track {key}"));
        }
        tracks.insert(key.into(), track);
    }
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    transitions.push(QuestTransition {
        world_change: None,
        key: COMPLETE.into(),
        quest: QUEST.into(),
        effects: vec![
            QuestEffect {
                track: READY.into(),
                from: QuestComparison::Eq(1),
                effect: QuestEffectKind::Set(1),
            },
            QuestEffect {
                track: CLAIMED.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            },
        ],
        completes: true,
        experience: None,
    });
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}
