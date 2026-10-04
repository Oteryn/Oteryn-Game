//! The QuestState content loader (QUEST-LOWER-1; QUEST-STATE-0 §13.2).
//!
//! `content/quests/missions/quest-state.json` is generated from the Quest definitions by
//! `tools/content-schema/quest-authoring/quest_state_lowering.py`: per quest, its tracks and
//! transitions under Oteryn keys, each with its source key kept as a source binding. It is
//! embedded at build time and [`load_embedded_quest_state`] parses it into a
//! [`QuestStateCatalogue`] for the server's content revision. Any unknown field, effect kind or
//! comparison, a record naming another quest, or a catalogue `QuestStateCatalogue::new`
//! refuses fails the whole load closed; nothing is skipped.
//!
//! An effect whose source `from` was not lowered exactly (`from_exact: false`: the generator
//! kept one comparison of a compound source condition) is not executable: it loads as
//! [`QuestEffectKind::Computed`], so its transition is refused `NOT_SUPPORTED` until the omitted
//! predicates are lowered, and never runs on a partial guard.
//!
//! The source keys and the NPC `requested_by` bindings stay beside the catalogue
//! ([`LoweredQuestState`]): they are content evidence for the callers that bind them
//! (NPC-QUEST-CONTENT-1) and never reach the store or the wire.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{
    QuestCatalogueError, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition,
};

const EMBEDDED: &str = include_str!("../../../../content/quests/missions/quest-state.json");
const SCHEMA: &str = "OTERYN_QUEST_STATE_LOWERING/v1";

/// The NPC dialogue a source transition was written from (format §3.2): the source NPC bundle
/// key, the player keywords and the dialogue topics around the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestRequestedBy {
    pub npc: String,
    pub keywords: Vec<QuestKeyword>,
    pub topics: Vec<i64>,
}

/// A player keyword: the word, or a text reference where the source phrase is not committed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum QuestKeyword {
    Word(String),
    TextRef(QuestTextRef),
}

/// The SHA-256 and byte length of a source phrase (LICENSE-ASSETS.md: no source text).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTextRef {
    pub sha256: String,
    pub length: u64,
    pub placeholders: Vec<String>,
}

/// The loaded catalogue with its content-side bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredQuestState {
    catalogue: QuestStateCatalogue,
    source_tracks: BTreeMap<String, String>,
    requested_by: BTreeMap<String, QuestRequestedBy>,
}

impl LoweredQuestState {
    #[must_use]
    pub fn catalogue(&self) -> &QuestStateCatalogue {
        &self.catalogue
    }

    /// The source key a track was lowered from.
    #[must_use]
    pub fn source_track(&self, track: &str) -> Option<&str> {
        self.source_tracks.get(track).map(String::as_str)
    }

    /// The NPC dialogue that requests a transition, when its source write was an NPC's.
    #[must_use]
    pub fn requested_by(&self, transition: &str) -> Option<&QuestRequestedBy> {
        self.requested_by.get(transition)
    }
}

/// Why the lowered quest content does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestLoadError {
    Json(String),
    Schema(String),
    ForeignRecord(String),
    Comparison(String),
    Effect(String),
    Catalogue(QuestCatalogueError),
}

// Every field is named, the unread ones as `IgnoredAny`, so an unknown field fails the load.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    #[serde(rename = "classification")]
    _classification: IgnoredAny,
    #[serde(rename = "family")]
    _family: IgnoredAny,
    #[serde(rename = "contract")]
    _contract: IgnoredAny,
    #[serde(rename = "authoring_sources")]
    _authoring_sources: IgnoredAny,
    #[serde(rename = "counts")]
    _counts: IgnoredAny,
    quests: Vec<Quest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Quest {
    quest: String,
    #[serde(rename = "source_quest")]
    _source_quest: IgnoredAny,
    #[serde(rename = "completion")]
    _completion: IgnoredAny,
    tracks: Vec<Track>,
    transitions: Vec<Transition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    key: String,
    quest: String,
    initial: i64,
    min: i64,
    max: i64,
    #[serde(rename = "bounds_basis")]
    _bounds_basis: IgnoredAny,
    source_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    key: String,
    quest: String,
    completes: bool,
    effects: Vec<Effect>,
    requested_by: Option<RequestedBy>,
    #[serde(rename = "source")]
    _source: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    track: String,
    from: Tagged,
    from_exact: bool,
    effect: Tagged,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tagged {
    #[serde(alias = "kind")]
    op: String,
    value: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedBy {
    npc: String,
    keywords: Vec<QuestKeyword>,
    topics: Vec<i64>,
}

/// Load the embedded lowered quest content for `content_revision`.
pub fn load_embedded_quest_state(
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    parse_quest_state(EMBEDDED, content_revision)
}

/// Parse one lowered quest content document for `content_revision`.
pub fn parse_quest_state(
    text: &str,
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let document: Document =
        serde_json::from_str(text).map_err(|error| QuestLoadError::Json(error.to_string()))?;
    if document.schema != SCHEMA {
        return Err(QuestLoadError::Schema(document.schema));
    }
    let mut tracks = Vec::new();
    let mut transitions = Vec::new();
    let mut source_tracks = BTreeMap::new();
    let mut requested_by = BTreeMap::new();
    for quest in document.quests {
        for track in quest.tracks {
            if track.quest != quest.quest {
                return Err(QuestLoadError::ForeignRecord(track.key));
            }
            source_tracks.insert(track.key.clone(), track.source_key);
            tracks.push(QuestTrack {
                key: track.key,
                quest: track.quest,
                initial: track.initial,
                min: track.min,
                max: track.max,
            });
        }
        for transition in quest.transitions {
            if transition.quest != quest.quest {
                return Err(QuestLoadError::ForeignRecord(transition.key));
            }
            let effects = transition
                .effects
                .into_iter()
                .map(|effect| {
                    let from = comparison(&effect.from)
                        .ok_or_else(|| QuestLoadError::Comparison(transition.key.clone()))?;
                    let kind = effect_kind(&effect.effect)
                        .ok_or_else(|| QuestLoadError::Effect(transition.key.clone()))?;
                    Ok(QuestEffect {
                        from,
                        effect: if effect.from_exact {
                            kind
                        } else {
                            QuestEffectKind::Computed
                        },
                        track: effect.track,
                    })
                })
                .collect::<Result<Vec<_>, QuestLoadError>>()?;
            if let Some(binding) = transition.requested_by {
                requested_by.insert(
                    transition.key.clone(),
                    QuestRequestedBy {
                        npc: binding.npc,
                        keywords: binding.keywords,
                        topics: binding.topics,
                    },
                );
            }
            transitions.push(QuestTransition {
                key: transition.key,
                quest: transition.quest,
                effects,
                completes: transition.completes,
            });
        }
    }
    let catalogue = QuestStateCatalogue::new(content_revision, tracks, transitions)
        .map_err(QuestLoadError::Catalogue)?;
    Ok(LoweredQuestState {
        catalogue,
        source_tracks,
        requested_by,
    })
}

fn comparison(from: &Tagged) -> Option<QuestComparison> {
    Some(match (from.op.as_str(), from.value) {
        ("ANY", None) => QuestComparison::Any,
        ("EQ", Some(value)) => QuestComparison::Eq(value),
        ("NE", Some(value)) => QuestComparison::Ne(value),
        ("LT", Some(value)) => QuestComparison::Lt(value),
        ("LE", Some(value)) => QuestComparison::Le(value),
        ("GT", Some(value)) => QuestComparison::Gt(value),
        ("GE", Some(value)) => QuestComparison::Ge(value),
        _ => return None,
    })
}

fn effect_kind(effect: &Tagged) -> Option<QuestEffectKind> {
    Some(match (effect.op.as_str(), effect.value) {
        ("SET", Some(value)) => QuestEffectKind::Set(value),
        ("ADD", Some(value)) => QuestEffectKind::Add(value),
        ("SET_NOW", None) => QuestEffectKind::SetNow,
        ("COMPUTED", None) => QuestEffectKind::Computed,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::{QuestRefusal, valid_quest_key};
    use super::*;

    const QUEST: &str = "oteryn:quest.a_father_s_burden_quest";
    const QUEST_LOG: &str = "oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log";
    const QUEST_LOG_NPC_1: &str =
        "oteryn:quest-transition/quest/u8_6/afathers_burden/quest_log/npc_1";

    fn embedded() -> LoweredQuestState {
        load_embedded_quest_state("content-1").expect("embedded quest state loads")
    }

    fn document() -> serde_json::Value {
        serde_json::from_str(EMBEDDED).expect("embedded quest state is JSON")
    }

    fn reparse(document: &serde_json::Value) -> Result<LoweredQuestState, QuestLoadError> {
        parse_quest_state(&document.to_string(), "content-1")
    }

    #[test]
    fn every_lowered_record_loads_under_an_oteryn_key() {
        let lowered = embedded();
        let document = document();
        let quests = document["quests"].as_array().expect("quests");
        let (mut tracks, mut transitions) = (0, 0);
        for quest in quests {
            let key = quest["quest"].as_str().expect("quest key");
            assert!(valid_quest_key(key), "{key}");
            assert!(lowered.catalogue().definition_hash(key).is_some(), "{key}");
            for track in quest["tracks"].as_array().expect("tracks") {
                let key = track["key"].as_str().expect("track key");
                let loaded = lowered.catalogue().track(key).expect("track loads");
                assert!(valid_quest_key(&loaded.key) && valid_quest_key(&loaded.quest));
                assert_eq!(lowered.source_track(key), track["source_key"].as_str());
                tracks += 1;
            }
            for transition in quest["transitions"].as_array().expect("transitions") {
                let key = transition["key"].as_str().expect("transition key");
                assert!(lowered.catalogue().transition(key).is_some(), "{key}");
                assert_eq!(
                    lowered.requested_by(key).is_some(),
                    !transition["requested_by"].is_null()
                );
                transitions += 1;
            }
        }
        assert_eq!(tracks, document["counts"]["tracks"]);
        assert_eq!(transitions, document["counts"]["transitions"]);
        assert!(tracks > 0 && transitions > 0);
    }

    #[test]
    fn a_lowered_npc_transition_keeps_its_source_and_dialogue_bindings() {
        let lowered = embedded();
        let track = lowered.catalogue().track(QUEST_LOG).expect("track");
        assert_eq!(
            (track.quest.as_str(), track.initial, track.min, track.max),
            (QUEST, 0, 0, 1)
        );
        assert_eq!(
            lowered.source_track(QUEST_LOG),
            Some("canary:quest-progress/quest/u8_6/afathers_burden/quest_log")
        );
        let transition = lowered
            .catalogue()
            .transition(QUEST_LOG_NPC_1)
            .expect("transition");
        assert_eq!(
            transition.effects,
            vec![QuestEffect {
                track: QUEST_LOG.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            }]
        );
        assert!(!transition.completes);
        assert_eq!(
            lowered.requested_by(QUEST_LOG_NPC_1),
            Some(&QuestRequestedBy {
                npc: "canary:npc/tereban_functions".into(),
                keywords: vec![QuestKeyword::Word("yes".into())],
                topics: vec![1],
            })
        );
        let changes = lowered
            .catalogue()
            .evaluate(transition, &BTreeMap::new(), 0)
            .expect("first write");
        assert_eq!((changes[0].before, changes[0].after), (0, 1));
    }

    #[test]
    fn computed_expressions_load_and_are_refused_not_supported() {
        let lowered = embedded();
        let document = document();
        let computed = document["quests"]
            .as_array()
            .expect("quests")
            .iter()
            .flat_map(|quest| quest["transitions"].as_array().expect("transitions"))
            .find(|transition| transition["effects"][0]["effect"]["kind"] == "COMPUTED")
            .expect("a computed transition");
        let transition = lowered
            .catalogue()
            .transition(computed["key"].as_str().expect("key"))
            .expect("loaded");
        assert_eq!(
            lowered
                .catalogue()
                .evaluate(transition, &BTreeMap::new(), 0),
            Err(QuestRefusal::NotSupported)
        );
    }

    #[test]
    fn an_inexact_source_guard_is_refused_not_supported() {
        let document = document();
        let (quest, index) = document["quests"]
            .as_array()
            .expect("quests")
            .iter()
            .enumerate()
            .flat_map(|(quest, entry)| {
                let transitions = entry["transitions"].as_array().expect("transitions");
                (0..transitions.len()).map(move |index| (quest, index))
            })
            .find(|&(quest, index)| {
                let effects = document["quests"][quest]["transitions"][index]["effects"]
                    .as_array()
                    .expect("effects");
                effects.iter().any(|effect| effect["from_exact"] == false)
                    && effects
                        .iter()
                        .all(|effect| effect["effect"]["kind"] != "COMPUTED")
            })
            .expect("an inexact transition with closed effects");
        let key = document["quests"][quest]["transitions"][index]["key"]
            .as_str()
            .expect("key")
            .to_owned();
        let evaluate = |lowered: &LoweredQuestState| {
            let transition = lowered.catalogue().transition(&key).expect("loaded");
            lowered
                .catalogue()
                .evaluate(transition, &BTreeMap::new(), 0)
                .err()
        };
        assert_eq!(evaluate(&embedded()), Some(QuestRefusal::NotSupported));

        // The same transition with its guard marked exact is executable again.
        let mut exact = document.clone();
        for effect in exact["quests"][quest]["transitions"][index]["effects"]
            .as_array_mut()
            .expect("effects")
        {
            effect["from_exact"] = true.into();
        }
        let exact = reparse(&exact).expect("exact guard loads");
        assert_ne!(evaluate(&exact), Some(QuestRefusal::NotSupported));
    }

    #[test]
    fn malformed_content_fails_the_whole_load_closed() {
        let mut bad = document();
        bad["schema"] = "OTHER/v1".into();
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Schema(_))));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["extra"] = 1.into();
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Json(_))));

        let mut bad = document();
        bad["quests"][0]["transitions"][0]["effects"][0]["from"] =
            serde_json::json!({"op": "ELAPSED", "value": 1});
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Comparison(_))));

        let mut bad = document();
        bad["quests"][0]["transitions"][0]["effects"][0]["effect"] =
            serde_json::json!({"kind": "SET"});
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Effect(_))));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["quest"] = "oteryn:quest.other".into();
        assert!(matches!(
            reparse(&bad),
            Err(QuestLoadError::ForeignRecord(_))
        ));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["key"] = "canary:quest-progress/leak".into();
        assert!(matches!(
            reparse(&bad),
            Err(QuestLoadError::Catalogue(QuestCatalogueError::InvalidKey(
                _
            )))
        ));

        assert!(matches!(
            parse_quest_state(EMBEDDED, "bad revision"),
            Err(QuestLoadError::Catalogue(
                QuestCatalogueError::InvalidContentRevision
            ))
        ));
    }
}
