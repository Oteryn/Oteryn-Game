//! Crystal 201189f8, creaturescripts_lower_roshamuul.lua:1-19.
use super::Counter;
use crate::quest::{QuestTrack, npc_rules::sandomo};

pub(super) const GATE: &str = "oteryn:quest-progress/roshamuul/lower-kill-credit";
const MAXIMUM: i64 = u32::MAX as i64;

pub(super) const COUNTERS: &[Counter] = &[
    Counter {
        transition: "oteryn:quest-transition/native-kills/roshamuul-frazzlemaw",
        creature: "oteryn:creature.frazzlemaw",
        quest: sandomo::QUEST,
        track: sandomo::TRACKS[0],
        gate: GATE,
        maximum: MAXIMUM,
        authored_stage: false,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/roshamuul-silencer",
        creature: "oteryn:creature.silencer",
        quest: sandomo::QUEST,
        track: sandomo::TRACKS[3],
        gate: GATE,
        maximum: MAXIMUM,
        authored_stage: false,
        world_change: None,
    },
];

pub(super) fn extend_tracks(tracks: &mut Vec<QuestTrack>) -> Result<(), String> {
    for (key, maximum) in std::iter::once((GATE, 0)).chain(
        COUNTERS
            .iter()
            .map(|counter| (counter.track, counter.maximum)),
    ) {
        let track = QuestTrack {
            key: key.into(),
            quest: sandomo::QUEST.into(),
            initial: 0,
            min: 0,
            max: maximum,
        };
        if let Some(existing) = tracks.iter().find(|existing| existing.key == key) {
            if existing != &track {
                return Err(format!("Roshamuul counter track conflict: {key}"));
            }
        } else {
            tracks.push(track);
        }
    }
    Ok(())
}
