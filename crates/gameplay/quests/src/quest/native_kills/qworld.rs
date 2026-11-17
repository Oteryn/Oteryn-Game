//! Authored personal death stages reuse the native counter and world predicate owners.
use super::Counter;
use crate::quest::QuestTrack;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "'de: 'static"))]
struct Document {
    schema: u32,
    objectives: Vec<Counter>,
    sources: Vec<String>,
}

pub(super) fn counters() -> Result<&'static [Counter], String> {
    static DOCUMENT: OnceLock<Result<Document, String>> = OnceLock::new();
    DOCUMENT
        .get_or_init(|| {
            let document: Document = serde_json::from_str(include_str!(
                "../../../../../../content/quests/native-death-objectives.json"
            ))
            .map_err(|error| error.to_string())?;
            if document.schema != 1
                || document.objectives.len() > 32
                || document.sources.is_empty()
                || document.objectives.iter().any(|counter| {
                    counter.maximum <= 0
                        || counter.maximum > 10_000
                        || [
                            counter.transition,
                            counter.creature,
                            counter.quest,
                            counter.track,
                            counter.gate,
                        ]
                        .iter()
                        .any(|key| !crate::quest::valid_quest_key(key))
                        || !matches!(counter.world_change, Some("fury_gates" | "grimvale"))
                })
            {
                return Err("invalid authored world-death objectives".into());
            }
            Ok(document)
        })
        .as_ref()
        .map(|document| document.objectives.as_slice())
        .map_err(Clone::clone)
}

pub(super) fn extend_tracks(tracks: &mut Vec<QuestTrack>) -> Result<(), String> {
    for counter in counters()? {
        for (key, max) in [(counter.gate, 1), (counter.track, counter.maximum)] {
            tracks.push(QuestTrack {
                key: key.into(),
                quest: counter.quest.into(),
                initial: 0,
                min: 0,
                max,
            });
        }
    }
    Ok(())
}
