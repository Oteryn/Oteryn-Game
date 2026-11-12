//! Canonical source altar operation shared by catalogue and admission ownership.
use super::{ACCESS_TRACK, QUEST, SACRIFICE};
use crate::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};

pub(crate) fn transition() -> QuestTransition {
    QuestTransition {
        world_change: None,
        key: SACRIFICE.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: ACCESS_TRACK.into(),
            from: QuestComparison::Between(0, 3),
            effect: QuestEffectKind::Add(1),
        }],
        completes: false,
        experience: None,
    }
}
