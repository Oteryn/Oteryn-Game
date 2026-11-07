//! The Isle of Evil: closed NPC text and existing QuestState transitions.
//! Every item, gold, achievement and teleport branch remains with its own owner.
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const QUEST: &str = "oteryn:quest.the_isle_of_evil_quest";
pub const PREFIX: &str = "oteryn:quest-progress/crystalserver/quest/u8_5/the_isle_of_evil/";
pub const KV_PREFIX: &str = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/";
const SOURCE: &str = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:";
const DATA: &str = include_str!("../../../../../content/quests/isle-of-evil.json");
const TEXT: &str = include_str!("../../../../../content/dialogues/quests/isle-of-evil.json");

pub mod dialogue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    key: String,
    initial: i64,
    min: i64,
    max: i64,
    source: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    track: String,
    from: Option<i64>,
    set: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    key: String,
    source: String,
    effects: Vec<Effect>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BugFix {
    question: String,
    answer: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    schema: String,
    quest: String,
    tracks: Vec<Track>,
    transitions: Vec<Transition>,
    mortimer_prior_keywords: Vec<String>,
    mortimer_static_keywords: Vec<String>,
    kv_lowering: String,
    scutty_bug_fix: BugFix,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub lines: Vec<String>,
    pub source: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Npc {
    key: String,
    greet: String,
    farewell: String,
    walkaway: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StaticKeyword {
    npc: String,
    keyword: String,
    reply: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    schema: String,
    source_revision: String,
    sources: Vec<SourceFile>,
    npcs: Vec<Npc>,
    replies: BTreeMap<String, Reply>,
    static_keywords: Vec<StaticKeyword>,
}

pub struct Book {
    data: Data,
    text: Text,
}

pub fn load() -> Result<&'static Book, &'static str> {
    static BOOK: OnceLock<Result<Book, String>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let data: Data = serde_json::from_str(DATA).map_err(|error| error.to_string())?;
        let text: Text = serde_json::from_str(TEXT).map_err(|error| error.to_string())?;
        let book = Book { data, text };
        book.validate()?;
        Ok(book)
    })
    .as_ref()
    .map_err(String::as_str)
}

#[must_use]
pub fn transition_key(suffix: &str) -> String {
    format!("oteryn:quest-transition/isle-of-evil/{suffix}")
}

impl Book {
    fn validate(&self) -> Result<(), String> {
        if self.data.schema != "OTERYN_ISLE_OF_EVIL_V1"
            || self.data.quest != QUEST
            || self.text.schema != "OTERYN_ISLE_OF_EVIL_DIALOGUE_V1"
            || self.text.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || self.data.tracks.len() != 13
            || self.data.transitions.len() != 9
            || self.text.npcs.len() != 5
            || self.text.replies.len() != 76
            || self.text.sources.len() != 7
            || self.data.kv_lowering.is_empty()
            || self.data.scutty_bug_fix.reason.is_empty()
            || self.data.scutty_bug_fix.question
                != "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589174504"
            || self.data.scutty_bug_fix.answer
                != "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728"
        {
            return Err("Isle source scope".into());
        }
        let mut keys = BTreeSet::new();
        for track in &self.data.tracks {
            if !keys.insert(&track.key)
                || (!track.key.starts_with(PREFIX) && !track.key.starts_with(KV_PREFIX))
                || track.min > track.initial
                || track.initial > track.max
                || !(track.source.starts_with(SOURCE)
                    || track.source.starts_with(
                        "Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:",
                    ))
            {
                return Err("Isle track scope/bounds".into());
            }
        }
        let mut transitions = BTreeSet::new();
        for transition in &self.data.transitions {
            let mut effects = BTreeSet::new();
            if !transitions.insert(&transition.key)
                || !transition.source.starts_with(SOURCE)
                || transition.effects.is_empty()
                || transition.effects.len() > 8
            {
                return Err("Isle transition scope".into());
            }
            for effect in &transition.effects {
                let Some(track) = self
                    .data
                    .tracks
                    .iter()
                    .find(|track| track.key == effect.track)
                else {
                    return Err("Isle effect track".into());
                };
                if !effects.insert(&effect.track)
                    || !(track.min..=track.max).contains(&effect.set)
                    || effect
                        .from
                        .is_some_and(|value| !(track.min..=track.max).contains(&value))
                {
                    return Err("Isle effect bounds".into());
                }
            }
        }
        if self.text.sources.iter().any(|source| {
            source.sha256.len() != 64
                || !(source.path.starts_with("data-global/npc/")
                    || source.path == "data/npclib/npc_system/npc_handler.lua")
        }) || self
            .text
            .replies
            .values()
            .any(|reply| reply.lines.is_empty() || !reply.source.starts_with(SOURCE))
            || self.text.npcs.iter().any(|npc| {
                !npc.key.starts_with("oteryn:npc.")
                    || [&npc.greet, &npc.farewell, &npc.walkaway]
                        .iter()
                        .any(|key| !self.text.replies.contains_key(*key))
            })
            || self
                .text
                .static_keywords
                .iter()
                .any(|keyword| !self.text.replies.contains_key(&keyword.reply))
        {
            return Err("Isle text scope".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn reply(&self, key: &str) -> Option<&Reply> {
        self.text.replies.get(key)
    }
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let book = load()?;
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    for track in &book.data.tracks {
        let expected = QuestTrack {
            key: track.key.clone(),
            quest: QUEST.into(),
            initial: track.initial,
            min: track.min,
            max: track.max,
        };
        match catalogue.track(&track.key) {
            Some(actual) if actual != &expected => {
                return Err(format!("Isle existing track conflict {}", track.key));
            }
            Some(_) => {}
            None => tracks.push(expected),
        }
    }
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for transition in &book.data.transitions {
        transitions.push(QuestTransition {
            world_change: None,
            key: transition_key(&transition.key),
            quest: QUEST.into(),
            effects: transition
                .effects
                .iter()
                .map(|effect| QuestEffect {
                    track: effect.track.clone(),
                    from: effect
                        .from
                        .map_or(QuestComparison::Any, QuestComparison::Eq),
                    effect: QuestEffectKind::Set(effect.set),
                })
                .collect(),
            completes: false,
            experience: None,
        });
    }
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
#[path = "isle_of_evil/tests.rs"]
mod tests;
