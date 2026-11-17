//! Kesar's final siege report, Crystal 201189f data-global/npc/kesar_the_younger_day.lua:157,241-255.
//! Earlier siege progression belongs to the quest's existing producer.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};

pub const NPC: &str = "oteryn:npc.kesar_the_younger_day";
pub const QUEST: &str = "oteryn:quest.the_order_of_the_lion_quest";
pub const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission";
pub const DOOR: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/augustin_door";
pub const COMPLETE: &str = "oteryn:quest-transition/hero-of-bounac/final-report";
pub const ACHIEVEMENT: &str = "oteryn:achievement/lionheart";

pub fn is_report(message: &str) -> bool {
    super::classic_talk::contains(message, "siege")
}

pub fn reply() -> Vec<String> {
    [
        "It is done. Drume's forces lie broken on the battlefield. You witnessed the fall of a great enemy of the Order of the Lion. ...",
        "You stood tall against the enemy forces and helped our cause wherever you could. ...",
        "I will therefore announce you the 'Hero of Bounac'. You are an honourable citizen of this town now. ...",
        "Feel free to roam Bounac as you please. This also means certain benefits when trading with our court trader Augustin. ...",
        "Whenever you're here for a visit, the large house to the south of Bounac is now completely at your disposal. ...",
        "And if we can persuade you into a longer stay perhaps, there is a small house further to the south of the island which may pique your interest. ...",
        "Thank you for your service to Bounac, Hero.",
    ].into_iter().map(str::to_owned).collect()
}

pub(super) fn transition() -> QuestTransition {
    QuestTransition {
        world_change: None,
        key: COMPLETE.into(),
        quest: QUEST.into(),
        effects: vec![
            QuestEffect {
                track: MISSION.into(),
                from: QuestComparison::Ge(5),
                effect: QuestEffectKind::Set(5),
            },
            QuestEffect {
                track: DOOR.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            },
        ],
        completes: true,
        experience: None,
    }
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    for key in [MISSION, DOOR] {
        if catalogue
            .track(key)
            .is_none_or(|track| track.quest != QUEST)
        {
            return Err(format!("Kesar source track {key}"));
        }
    }
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    transitions.push(transition());
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}
