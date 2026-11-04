//! Liquid Black's two native NPC branches use the existing QuestState writer.
//! Crystal201189f scrutinon.lua:56-74 and spectulus.lua:379-414; full text and
//! both compound state changes are generated from that pin, never Lua execution.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u9_4/liquid_black_quest/questline";
pub const VISITOR: &str = "oteryn:quest-progress/quest/u9_4/liquid_black_quest/visitor";
pub const START: &str = "oteryn:quest-transition/liquid-black/visitor-start";
pub const ACCESS: &str = "oteryn:quest-transition/liquid-black/machine-access";
const DIALOGUE: &str = include_str!("../../../../../content/dialogues/liquid-black.json");
const STATE: &str = include_str!("../../../../../content/quests/liquid-black-npc-state.json");
const PIN: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Npc {
    Scrutinon,
    Spectulus,
}

impl Npc {
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "oteryn:npc.scrutinon" => Some(Self::Scrutinon),
            "oteryn:npc.spectulus" => Some(Self::Spectulus),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    ScrutinonVisitor,
    Spectulus21,
    Spectulus22,
    Spectulus23,
    Spectulus24,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    ScrutinonQuirefang,
    ScrutinonVisitor,
    ScrutinonName,
    SpectulusMachine,
    #[serde(rename = "spectulus_yes_21")]
    SpectulusYes21,
    #[serde(rename = "spectulus_yes_22")]
    SpectulusYes22,
    #[serde(rename = "spectulus_yes_23")]
    SpectulusYes23,
    SpectulusReturned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub reply: Option<Reply>,
    pub transition: Option<&'static str>,
    pub topic: Option<Topic>,
}

// Crystal data/npclib/npc.lua:13-20. Lua's %w is ASCII alphanumeric here;
// there is no trailing boundary, and one word-internal occurrence rejects it.
fn contains(message: &str, keyword: &str) -> bool {
    message == keyword
        || (message.contains(keyword)
            && !message.match_indices(keyword).any(|(start, _)| {
                start > 0 && message.as_bytes()[start - 1].is_ascii_alphanumeric()
            }))
}

#[must_use]
pub fn choose(
    npc: Npc,
    message: &str,
    topic: Option<Topic>,
    questline: i64,
    visitor: i64,
) -> Option<Choice> {
    let message = message.to_ascii_lowercase();
    let reply = |reply, topic| {
        Some(Choice {
            reply: Some(reply),
            transition: None,
            topic,
        })
    };
    match npc {
        Npc::Scrutinon => {
            // npc_system/keyword_handler.lua:58 uses an unbounded string.find;
            // npc_handler.lua:448 tries StdModule before the default callback.
            if message.contains("name") {
                reply(Reply::ScrutinonName, topic)
            } else if super::quests_2_focus::intercepts(&message) {
                None
            } else if contains(&message, "quirefang") {
                reply(Reply::ScrutinonQuirefang, Some(Topic::ScrutinonVisitor))
            } else if topic == Some(Topic::ScrutinonVisitor) && contains(&message, "visitor") {
                Some(Choice {
                    reply: Some(Reply::ScrutinonVisitor),
                    transition: (questline < 1).then_some(START),
                    topic: None,
                })
            } else {
                None
            }
        }
        Npc::Spectulus => {
            if super::quests_2_focus::intercepts(&message) {
                return None;
            }
            if contains(&message, "machine") && visitor == 3 {
                reply(Reply::SpectulusMachine, Some(Topic::Spectulus21))
            } else if contains(&message, "yes") && visitor == 3 {
                match topic {
                    Some(Topic::Spectulus21) => {
                        reply(Reply::SpectulusYes21, Some(Topic::Spectulus22))
                    }
                    Some(Topic::Spectulus22) => {
                        reply(Reply::SpectulusYes22, Some(Topic::Spectulus23))
                    }
                    Some(Topic::Spectulus23) => Some(Choice {
                        reply: Some(Reply::SpectulusYes23),
                        transition: Some(ACCESS),
                        topic: Some(Topic::Spectulus24),
                    }),
                    _ => None,
                }
            } else if !contains(&message, "yes") && contains(&message, "no") && topic.is_some() {
                // spectulus.lua:360-376 clears all source topics;21-24 have no refusal text.
                Some(Choice {
                    reply: None,
                    transition: None,
                    topic: None,
                })
            } else {
                None
            }
        }
    }
}

#[derive(Deserialize)]
struct Document {
    schema: String,
    family: String,
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    declaration: Declaration,
}

#[derive(Deserialize)]
struct Declaration {
    identity: Identity,
    keywords: Vec<Keyword>,
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Identity {
    key: String,
}

#[derive(Deserialize)]
struct Keyword {
    key: Reply,
    reply: Vec<String>,
}

#[derive(Deserialize)]
struct Field {
    field_path: String,
    value: Text,
}

#[derive(Deserialize)]
struct Text {
    value: String,
}

#[derive(Deserialize)]
struct Source {
    repository: String,
    revision: String,
    path: String,
}

fn parse_dialogue(text: &str) -> Result<Vec<Keyword>, String> {
    let document: Document = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if document.schema != "OTERYN_DIALOGUE_AUTHORING_SHARD/v1"
        || document.family != "Dialogue"
        || document.records.len() != 2
    {
        return Err("Liquid Black dialogue envelope".into());
    }
    let mut result = Vec::new();
    let mut npcs = BTreeSet::new();
    for record in document.records {
        let declaration = record.declaration;
        let expected = match declaration.identity.key.as_str() {
            "oteryn:dialogue.npc.scrutinon" => "data-global/npc/scrutinon.lua",
            "oteryn:dialogue.npc.spectulus" => "data-global/npc/spectulus.lua",
            _ => return Err("Liquid Black dialogue NPC".into()),
        };
        if !npcs.insert(declaration.identity.key) {
            return Err("Liquid Black duplicate NPC".into());
        }
        let source = declaration
            .fields
            .iter()
            .find(|field| field.field_path == "oteryn:source.quest.liquid_black.dialogue")
            .ok_or("Liquid Black dialogue source")?;
        let source: Source =
            serde_json::from_str(&source.value.value).map_err(|error| error.to_string())?;
        if source.repository != "zimbadev/crystalserver"
            || source.revision != PIN
            || source.path != expected
        {
            return Err("Liquid Black dialogue pin".into());
        }
        result.extend(declaration.keywords);
    }
    let keys: BTreeSet<_> = result.iter().map(|entry| entry.key).collect();
    if result.len() != 8
        || keys.len() != 8
        || result.iter().any(|entry| {
            entry.reply.is_empty()
                || entry.reply.len() > 6
                || entry
                    .reply
                    .iter()
                    .any(|line| line.is_empty() || line.len() > 2048 || line.contains(['\0', '\r']))
        })
    {
        return Err("Liquid Black dialogue replies".into());
    }
    Ok(result)
}

#[must_use]
pub fn reply(key: Reply) -> Option<&'static [String]> {
    static DATA: OnceLock<Result<Vec<Keyword>, String>> = OnceLock::new();
    DATA.get_or_init(|| parse_dialogue(DIALOGUE))
        .as_ref()
        .ok()?
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.reply.as_slice())
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    let extra = super::loader::parse_quest_state(STATE, &catalogue.content_revision)
        .map_err(|error| format!("{error:?}"))?;
    let mut tracks = catalogue.tracks.values().cloned().collect::<Vec<_>>();
    for track in extra.catalogue().tracks.values() {
        if let Some(existing) = tracks.iter().find(|existing| existing.key == track.key) {
            if existing != track {
                return Err(format!("Liquid Black conflicting track {}", track.key));
            }
        } else {
            tracks.push(track.clone());
        }
    }
    let mut transitions = catalogue.transitions.values().cloned().collect::<Vec<_>>();
    transitions.extend(extra.catalogue().transitions.values().cloned());
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn source_focus_precedes_liquid_default_callback_but_not_earlier_name() {
        // Crystal201189f scrutinon.lua:92 registers name before FocusModule
        // at105; npc_handler.lua:446-451 runs both before the default callback.
        for message in ["visitor hi", "visitor hello", "visitor trade"] {
            assert_eq!(
                choose(Npc::Scrutinon, message, Some(Topic::ScrutinonVisitor), 0, 0),
                None,
                "FocusModule must intercept {message}",
            );
        }
        assert_eq!(
            choose(
                Npc::Scrutinon,
                "name hi",
                Some(Topic::ScrutinonVisitor),
                0,
                0
            ),
            Some(Choice {
                reply: Some(Reply::ScrutinonName),
                transition: None,
                topic: Some(Topic::ScrutinonVisitor),
            }),
        );
    }

    #[test]
    fn visitor_requires_the_source_topic_and_does_not_reset_completed_access() {
        assert_eq!(choose(Npc::Scrutinon, "visitor", None, -1, 0), None);
        let first = choose(
            Npc::Scrutinon,
            "visitor",
            Some(Topic::ScrutinonVisitor),
            -1,
            0,
        );
        assert_eq!(first.and_then(|choice| choice.transition), Some(START));
        let repeat = choose(
            Npc::Scrutinon,
            "visitor",
            Some(Topic::ScrutinonVisitor),
            1,
            5,
        );
        assert_eq!(repeat.and_then(|choice| choice.transition), None);
        assert_eq!(
            choose(Npc::Scrutinon, "quirefang visitor", None, -1, 0)
                .and_then(|choice| choice.reply),
            Some(Reply::ScrutinonQuirefang)
        );
        assert_eq!(
            choose(Npc::Scrutinon, "rename quirefang", None, -1, 0).and_then(|choice| choice.reply),
            Some(Reply::ScrutinonName)
        );
    }

    #[test]
    fn spectulus_requires_three_confirmations_and_visitor_three() {
        let mut topic =
            choose(Npc::Spectulus, "machine", None, 1, 3).and_then(|choice| choice.topic);
        assert_eq!(topic, Some(Topic::Spectulus21));
        for expected in [Topic::Spectulus22, Topic::Spectulus23] {
            let choice = choose(Npc::Spectulus, "yes", topic, 1, 3);
            assert_eq!(choice.and_then(|value| value.transition), None);
            topic = choice.and_then(|value| value.topic);
            assert_eq!(topic, Some(expected));
        }
        assert_eq!(
            choose(Npc::Spectulus, "yes", topic, 1, 3).and_then(|choice| choice.transition),
            Some(ACCESS)
        );
        for visitor in [-1, 0, 1, 2, 4, 5] {
            assert_eq!(choose(Npc::Spectulus, "machine", None, 1, visitor), None);
            assert_eq!(choose(Npc::Spectulus, "yes", topic, 1, visitor), None);
        }
        assert_eq!(choose(Npc::Spectulus, "yes", None, 1, 3), None);
        assert_eq!(
            choose(Npc::Spectulus, "no", topic, 1, 3).and_then(|choice| choice.topic),
            None
        );
    }

    #[test]
    fn source_text_is_complete_and_does_not_create_a_gold_reward() -> Result<(), String> {
        assert_eq!(parse_dialogue(DIALOGUE)?.len(), 8);
        assert_eq!(reply(Reply::SpectulusYes21).ok_or("yes21")?.len(), 6);
        assert!(reply(Reply::SpectulusYes22).ok_or("yes22")?[2].contains("take this gold"));
        let state = super::super::loader::parse_quest_state(STATE, "source")
            .map_err(|error| format!("{error:?}"))?;
        let transition = state.catalogue().transition(ACCESS).ok_or("access")?;
        assert_eq!(transition.experience, None);
        assert_eq!(transition.effects.len(), 1);
        assert_eq!(transition.effects[0].track, VISITOR);
        Ok(())
    }

    #[test]
    fn compound_start_and_access_are_repeat_safe_in_the_real_catalogue() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("liquid-black")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let start = catalogue.transition(START).ok_or("start")?;
        let mut tracks = BTreeMap::new();
        for change in catalogue
            .evaluate(start, &tracks, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            tracks.insert(change.track, change.after);
        }
        assert_eq!(tracks.get(QUESTLINE), Some(&1));
        assert_eq!(tracks.get(VISITOR), Some(&1));
        assert_eq!(
            catalogue.evaluate(start, &tracks, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        let access = catalogue.transition(ACCESS).ok_or("access")?;
        assert_eq!(
            catalogue.evaluate(access, &tracks, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        tracks.insert(VISITOR.into(), 3);
        let changes = catalogue
            .evaluate(access, &tracks, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].after, 4);
        Ok(())
    }
}
