//! Lifetime reports reuse Quest tracks and receipts; no reset-created track names.
//!
//! Lumberjack: retained TibiaWiki page76247/rev893906, npc_conditions:9-13,
//! body SHA256 a31cf523648ff168e34f12a31429a05d8db42be4e7a0daa6df49dc6e32355ce2.
//! Talk to Chip before reporting to Queen Eloise; Whistle-Blower is the reward.
//! Exact dialogue words are absent, so the explicit conservative selector accepts
//! only logging/wood from Chip and Chip/lumberjack from Queen Eloise.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTrackChange, QuestTransition,
};
use std::collections::BTreeMap;

pub const QUEST: &str = "oteryn:quest.world_change_lumberjack";
pub const TRACK: &str = "oteryn:quest-progress/world-change/lumberjack/report";
pub const CHIP: &str = "oteryn:npc.chip";
pub const QUEEN: &str = "oteryn:npc.queen_eloise";
pub const CHANGE: &str = "lumberjack";
pub const TALK: &str = "oteryn:quest-transition/world-change/lumberjack/talk-chip";
pub const REPORT: &str = "oteryn:quest-transition/world-change/lumberjack/report-queen";
pub const ACHIEVEMENT: &str = "oteryn:achievement/whistle_blower";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TalkChip,
    ReportQueen,
}
impl Action {
    pub const fn npc(self) -> &'static str {
        match self {
            Self::TalkChip => CHIP,
            Self::ReportQueen => QUEEN,
        }
    }
    pub const fn transition(self) -> &'static str {
        match self {
            Self::TalkChip => TALK,
            Self::ReportQueen => REPORT,
        }
    }
    pub const fn requires_selected_npc(self) -> bool {
        matches!(self, Self::TalkChip)
    }
    pub const fn reply(self) -> &'static str {
        match self {
            Self::TalkChip => "I cut wood here. Queen Eloise would probably object to my logging.",
            Self::ReportQueen => {
                "Thank you for reporting Chip's logging. You have fulfilled your duty to Carlin."
            }
        }
    }
}
pub fn for_transition(key: &str) -> Option<Action> {
    match key {
        TALK => Some(Action::TalkChip),
        REPORT => Some(Action::ReportQueen),
        _ => None,
    }
}
/// Pure choice only. The native NPC owner supplies the current conversation;
/// the durable Quest owner rechecks both predecessor and source admission.
pub fn select(npc: &str, text: &str, tracks: &BTreeMap<String, i64>) -> Option<Action> {
    let stage = tracks.get(TRACK).copied().unwrap_or(0);
    let text = text.trim().to_ascii_lowercase();
    match (npc, stage, text.as_str()) {
        (CHIP, 0, "wood" | "logging") => Some(Action::TalkChip),
        (QUEEN, 1, "chip" | "lumberjack") => Some(Action::ReportQueen),
        _ => None,
    }
}
/// Covers renamed transitions as well: no generic writer may mutate this track
/// without the current native NPC context required by its source action.
pub fn requires_native_source(catalogue: &QuestStateCatalogue, key: &str) -> bool {
    for_transition(key).is_some()
        || catalogue.transition(key).is_some_and(|transition| {
            transition
                .effects
                .iter()
                .any(|effect| effect.track == TRACK)
        })
}
pub fn report_completed(changes: &[QuestTrackChange]) -> bool {
    changes.len() == 1
        && changes[0].track == TRACK
        && changes[0].before == 1
        && changes[0].after == 2
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    tracks.push(QuestTrack {
        key: TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: 2,
    });
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for (action, before, after) in [(Action::TalkChip, 0, 1), (Action::ReportQueen, 1, 2)] {
        transitions.push(QuestTransition {
            key: action.transition().into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Eq(before),
                effect: QuestEffectKind::Set(after),
            }],
            completes: matches!(action, Action::ReportQueen),
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_requires_chip_and_is_a_bounded_lifetime_completion() -> Result<(), String> {
        let catalogue = extend(QuestStateCatalogue::empty("test").ok_or("empty")?)?;
        assert!(requires_native_source(&catalogue, TALK));
        assert!(requires_native_source(&catalogue, REPORT));
        assert!(!requires_native_source(
            &catalogue,
            "oteryn:quest-transition/unrelated"
        ));
        let mut tracks = BTreeMap::new();
        assert_eq!(select(QUEEN, "chip", &tracks), None);
        assert_eq!(select(CHIP, "logging", &tracks), Some(Action::TalkChip));
        for (action, expected) in [(Action::TalkChip, 1), (Action::ReportQueen, 2)] {
            let transition = catalogue
                .transition(action.transition())
                .ok_or("transition")?;
            let changes = catalogue
                .evaluate(transition, &tracks, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(report_completed(&changes), expected == 2);
            for change in changes {
                tracks.insert(change.track, change.after);
            }
            assert_eq!(tracks.get(TRACK), Some(&expected));
            assert!(catalogue.evaluate(transition, &tracks, 0).is_err());
        }
        assert_eq!(select(CHIP, "wood", &tracks), None);
        assert_eq!(select(QUEEN, "chip", &tracks), None);
        assert_eq!(select("oteryn:npc.impostor", "chip", &tracks), None);
        Ok(())
    }
}
