//! Crystal 201189f, creaturescripts_fugue_kill.lua:4-9.
//! Uses the existing most-damage principal and observed source PartySnapshot.
use super::{Counter, QuestComparison, QuestEffect, QuestEffectKind};

pub(super) const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission";
pub(super) const SPAWNED: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/fugue_spawned";
pub(super) const COUNTERS: &[Counter] = &[Counter {
    transition: "oteryn:quest-transition/native-kills/fugue",
    creature: "oteryn:creature.fugue",
    quest: "oteryn:quest.the_order_of_the_lion_quest",
    track: MISSION,
    gate: MISSION,
    maximum: 3,
    world_change: None,
}];
pub(super) fn matches(counter: &Counter) -> bool {
    counter.transition == COUNTERS[0].transition
}
pub(super) fn effects() -> Vec<QuestEffect> {
    vec![
        QuestEffect {
            track: MISSION.into(),
            from: QuestComparison::Eq(2),
            effect: QuestEffectKind::Set(3),
        },
        QuestEffect {
            track: SPAWNED.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(0),
        },
    ]
}
