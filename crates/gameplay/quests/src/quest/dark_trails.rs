//! Dark Trails storage dialogues, transcribed from Crystal 201189f.
//! Inventory, cross-quest writes, appearances and boss encounters have separate owners.
use std::collections::BTreeMap;

use serde::Deserialize;

pub const QUEST: &str = "oteryn:quest.dark_trails_quest";
pub const TRACK_PREFIX: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/";
pub const EMBEDDED: &str = include_str!("../../../../../content/quests/dark-trails.json");
pub const DIALOGUE: &str = include_str!("../../../../../content/dialogues/quests/dark-trails.json");

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub key: String,
    pub initial: i64,
    pub minimum: i64,
    pub maximum: i64,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Set {
    pub track: String,
    pub value: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub key: String,
    pub npc: String,
    pub keyword: String,
    pub topic: Option<u16>,
    pub next_topic: Option<u16>,
    pub greeting_only: bool,
    pub guards: Vec<Guard>,
    pub sets: Vec<Set>,
    pub reply: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub lines: Vec<String>,
    pub source: String,
}

#[derive(Debug)]
pub struct Book {
    pub tracks: Vec<Track>,
    pub rules: Vec<Rule>,
    pub replies: BTreeMap<String, Reply>,
    pub branches: BTreeMap<String, Vec<String>>,
}

pub fn load() -> Result<Book, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Data {
        schema: String,
        quest: String,
        tracks: Vec<Track>,
        rules: Vec<Rule>,
        branches: BTreeMap<String, Vec<String>>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Dialogue {
        schema: String,
        replies: BTreeMap<String, Reply>,
    }
    let data: Data = serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
    let text: Dialogue = serde_json::from_str(DIALOGUE).map_err(|e| e.to_string())?;
    if data.schema != "OTERYN_DARK_TRAILS_V1"
        || data.quest != QUEST
        || text.schema != "OTERYN_DARK_TRAILS_DIALOGUE_V1"
    {
        return Err("Dark Trails schema".into());
    }
    let book = Book {
        tracks: data.tracks,
        rules: data.rules,
        replies: text.replies,
        branches: data.branches,
    };
    book.validate()?;
    Ok(book)
}

#[must_use]
pub fn transition_key(rule: &str) -> String {
    format!("oteryn:quest-transition/dark-trails/{rule}")
}

#[must_use]
pub fn message_contains(message: &str, keyword: &str) -> bool {
    // Crystal 201189f data/npclib/npc.lua:13-20. These authored keywords
    // contain no Lua pattern metacharacters. A prefixed occurrence anywhere
    // rejects the message, even when another occurrence is a separate word.
    if keyword.is_empty() {
        return false;
    }
    let message = message.to_ascii_lowercase();
    let keyword = keyword.to_ascii_lowercase();
    if message == keyword {
        return true;
    }
    let mut found = false;
    for (offset, candidate) in message.as_bytes().windows(keyword.len()).enumerate() {
        if candidate != keyword.as_bytes() {
            continue;
        }
        found = true;
        if offset > 0 && message.as_bytes()[offset - 1].is_ascii_alphanumeric() {
            return false;
        }
    }
    found
}

impl Book {
    #[must_use]
    pub fn select(
        &self,
        npc: &str,
        message: &str,
        topic: u16,
        tracks: &BTreeMap<String, i64>,
    ) -> Option<&Rule> {
        let first_branch = self.branches.get(npc).and_then(|keywords| {
            keywords
                .iter()
                .find(|keyword| message_contains(message, keyword))
        });
        self.rules.iter().find(|rule| {
            rule.npc == npc
                && first_branch.is_none_or(|keyword| keyword == &rule.keyword)
                && message_contains(message, &rule.keyword)
                && rule.topic.is_none_or(|required| required == topic)
                && rule.guards.iter().all(|guard| {
                    let initial = self
                        .tracks
                        .iter()
                        .find(|track| track.key == guard.track)
                        .map_or(-1, |track| track.initial);
                    let value = tracks.get(&guard.track).copied().unwrap_or(initial);
                    (guard.minimum..=guard.maximum).contains(&value)
                })
        })
    }

    fn validate(&self) -> Result<(), String> {
        use std::collections::BTreeSet;
        const SOURCE: &str = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:";
        let mut track_keys = BTreeSet::new();
        for track in &self.tracks {
            if !track.key.starts_with(TRACK_PREFIX)
                || !track_keys.insert(&track.key)
                || track.minimum > track.initial
                || track.initial > track.maximum
                || !track.source.starts_with(SOURCE)
            {
                return Err(format!("invalid Dark Trails track {}", track.key));
            }
        }
        let mut keys = BTreeSet::new();
        for rule in &self.rules {
            let mut guards = BTreeSet::new();
            let mut sets = BTreeSet::new();
            if !keys.insert(&rule.key)
                || rule.key.is_empty()
                || !rule.npc.starts_with("oteryn:npc.")
                || rule.keyword.is_empty()
                || !rule
                    .keyword
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b' ')
                || rule.sources.is_empty()
                || rule
                    .sources
                    .iter()
                    .any(|source| !source.starts_with(SOURCE))
            {
                return Err(format!("invalid Dark Trails rule {}", rule.key));
            }
            let reply = self
                .replies
                .get(&rule.reply)
                .ok_or_else(|| format!("missing Dark Trails reply {}", rule.reply))?;
            if reply.lines.is_empty()
                || reply.lines.iter().any(String::is_empty)
                || !reply.source.starts_with(SOURCE)
            {
                return Err(format!("invalid Dark Trails reply {}", rule.reply));
            }
            for guard in &rule.guards {
                let track = self
                    .tracks
                    .iter()
                    .find(|track| track.key == guard.track)
                    .ok_or_else(|| format!("unknown Dark Trails guard {}", guard.track))?;
                if !guards.insert(&guard.track)
                    || guard.minimum > guard.maximum
                    || guard.minimum < track.minimum
                    || guard.maximum > track.maximum
                {
                    return Err(format!("invalid Dark Trails guard {}", guard.track));
                }
            }
            for set in &rule.sets {
                let track = self
                    .tracks
                    .iter()
                    .find(|track| track.key == set.track)
                    .ok_or_else(|| format!("unknown Dark Trails set {}", set.track))?;
                if !sets.insert(&set.track) || !(track.minimum..=track.maximum).contains(&set.value)
                {
                    return Err(format!("invalid Dark Trails set {}", set.track));
                }
            }
        }
        Ok(())
    }

    pub fn reply(&self, rule: &Rule) -> Result<String, String> {
        self.replies
            .get(&rule.reply)
            .map(|reply| reply.lines.join("\n"))
            .ok_or_else(|| format!("missing reply {}", rule.reply))
    }
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let book = load()?;
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    for track in &book.tracks {
        let expected = QuestTrack {
            key: track.key.clone(),
            quest: QUEST.into(),
            initial: track.initial,
            min: track.minimum,
            max: track.maximum,
        };
        if let Some(existing) = catalogue.track(&track.key) {
            if existing != &expected {
                return Err(format!(
                    "Dark Trails track ownership/bounds conflict {}",
                    track.key
                ));
            }
        } else {
            tracks.push(expected);
        }
    }
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for rule in &book.rules {
        if rule.sets.is_empty() {
            continue;
        }
        let mut effects = BTreeMap::new();
        for guard in &rule.guards {
            effects.insert(
                guard.track.clone(),
                QuestEffect {
                    track: guard.track.clone(),
                    from: QuestComparison::Between(guard.minimum, guard.maximum),
                    effect: QuestEffectKind::Add(0),
                },
            );
        }
        for set in &rule.sets {
            let track = book
                .tracks
                .iter()
                .find(|track| track.key == set.track)
                .ok_or_else(|| format!("unknown Dark Trails track {}", set.track))?;
            let entry = effects
                .entry(set.track.clone())
                .or_insert_with(|| QuestEffect {
                    track: set.track.clone(),
                    from: QuestComparison::Between(track.minimum, track.maximum),
                    effect: QuestEffectKind::Add(0),
                });
            entry.effect = QuestEffectKind::Set(set.value);
        }
        transitions.push(QuestTransition {
            world_change: None,
            key: transition_key(&rule.key),
            quest: QUEST.into(),
            effects: effects.into_values().collect(),
            completes: false,
            experience: None,
        });
    }
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
#[path = "dark_trails/tests.rs"]
mod tests;
