//! Mazarius' ordered source dialogue and the exact hand-in recipe.
//!
//! zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/npc/mazarius.lua:54 and data/npclib/npc.lua:13.
//! The caller owns focus, session generation, inventory admission and the atomic
//! item/quest commit. Selecting an Exchange does not grant access or remove items.

use serde::Deserialize;

pub const QUEST: &str = "oteryn:quest.ferumbras_ascension_quest";
pub const NPC: &str = "oteryn:npc.mazarius";
const EMBEDDED: &str = include_str!("../../../../../content/dialogues/ferumbras-mazarius.json");
const SOURCE_PREFIX: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/mazarius.lua:";
const KEYWORD_SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceItem {
    pub source_id: u32,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorageWrite {
    pub storage: u32,
    pub value: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub remove: SourceItem,
    pub give: SourceItem,
    pub writes: Vec<StorageWrite>,
    pub refusal: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub keywords: Vec<String>,
    pub topic: Option<u8>,
    pub access_below: Option<i64>,
    /// None preserves the current topic; it does not reset it to zero.
    pub next_topic: Option<u8>,
    pub reply: Vec<String>,
    pub exchange: Option<Exchange>,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dialogue {
    schema: u32,
    npc: String,
    quest: String,
    source: String,
    keyword_source: String,
    pub greet: String,
    pub farewell: String,
    pub walkaway: String,
    pub branches: Vec<Branch>,
}

impl Dialogue {
    pub fn load() -> Result<Self, String> {
        Self::parse(EMBEDDED)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let data: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let valid_text = |s: &str| !s.is_empty() && s.len() <= 2048 && !s.contains('\0');
        let valid_lines = |lines: &[String]| {
            !lines.is_empty() && lines.len() <= 16 && lines.iter().all(|s| valid_text(s))
        };
        let source = |s: &str| {
            s.strip_prefix(SOURCE_PREFIX)
                .and_then(|line| line.parse::<u16>().ok())
                .is_some_and(|line| (54..=191).contains(&line))
        };
        if data.schema != 1
            || data.npc != NPC
            || data.quest != QUEST
            || !source(&data.source)
            || data.keyword_source != KEYWORD_SOURCE
            || !valid_text(&data.greet)
            || !valid_text(&data.farewell)
            || !valid_text(&data.walkaway)
            || data.branches.is_empty()
            || data.branches.len() > 32
        {
            return Err("invalid Ferumbras dialogue header".into());
        }
        let final_index = data.branches.len() - 1;
        for (index, branch) in data.branches.iter().enumerate() {
            let fallback = branch.keywords.is_empty();
            if fallback != (index == final_index)
                || (fallback && (branch.topic.is_some() || branch.access_below.is_some()))
                || branch.keywords.len() > 2
                || branch.keywords.iter().any(|s| !supported_keyword(s))
                || branch.topic.is_some_and(|topic| ![2, 3].contains(&topic))
                || branch
                    .next_topic
                    .is_some_and(|topic| ![0, 2, 3].contains(&topic))
                || branch.access_below.is_some_and(|value| value != 1)
                || !valid_lines(&branch.reply)
                || !source(&branch.source)
            {
                return Err("invalid Ferumbras dialogue branch".into());
            }
            if let Some(exchange) = &branch.exchange {
                // Only the source recipe can request item-owner work from this adapter.
                // In particular, access is not a once-only guard in mazarius.lua:97.
                let writes = [
                    StorageWrite {
                        storage: 45211,
                        value: 1,
                    },
                    StorageWrite {
                        storage: 45216,
                        value: 1,
                    },
                    StorageWrite {
                        storage: 45217,
                        value: 1,
                    },
                    StorageWrite {
                        storage: 45218,
                        value: 1,
                    },
                ];
                if branch.topic != Some(3)
                    || branch.access_below.is_some()
                    || branch.next_topic != Some(0)
                    || branch.keywords != ["yes"]
                    || exchange.remove
                        != (SourceItem {
                            source_id: 6499,
                            count: 30,
                        })
                    || exchange.give
                        != (SourceItem {
                            source_id: 22182,
                            count: 1,
                        })
                    || exchange.writes != writes
                    || !valid_lines(&exchange.refusal)
                {
                    return Err("invalid Ferumbras exchange".into());
                }
            }
        }
        Ok(data)
    }

    /// First matching source elseif wins, including a guarded yes before essence.
    pub fn choose(&self, message: &str, topic: u8, access: i64) -> Option<&Branch> {
        self.branches.iter().find(|branch| {
            branch.topic.is_none_or(|required| topic == required)
                && branch.access_below.is_none_or(|limit| access < limit)
                && (branch.keywords.is_empty()
                    || branch
                        .keywords
                        .iter()
                        .any(|word| source_keyword_matches(message, word)))
        })
    }
}

fn supported_keyword(keyword: &str) -> bool {
    !keyword.is_empty()
        && keyword.len() <= 32
        && (keyword == "demi-plane"
            || keyword
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || matches!(byte, b' ' | b'\'')))
}

/// Bounded translation of MsgContains for this NPC's literal keyword vocabulary.
/// Source accepts suffixes ("yesterday" matches "yes") and rejects the whole
/// message if ANY matching occurrence is preceded by an alphanumeric byte.
/// Lua's '-' repetition operator applies to the sole pattern "demi-plane".
pub fn source_keyword_matches(message: &str, keyword: &str) -> bool {
    if !supported_keyword(keyword) {
        return false;
    }
    let message = message.to_ascii_lowercase();
    let keyword = keyword.to_ascii_lowercase();
    if message == keyword {
        return true;
    }
    let offsets: Vec<_> = if keyword == "demi-plane" {
        message
            .match_indices("dem")
            .filter_map(|(offset, _)| {
                let mut tail = message.get(offset + 3..)?;
                tail = tail.trim_start_matches('i');
                tail.starts_with("plane").then_some(offset)
            })
            .collect()
    } else {
        message
            .match_indices(keyword.as_str())
            .map(|(offset, _)| offset)
            .collect()
    };
    !offsets.is_empty()
        && offsets.iter().all(|offset| {
            offset
                .checked_sub(1)
                .and_then(|previous| message.as_bytes().get(previous))
                .is_none_or(|previous| !previous.is_ascii_alphanumeric())
        })
}
#[cfg(test)]
mod tests {
    use super::*;

    fn dialogue() -> Result<Dialogue, String> {
        Dialogue::load()
    }

    #[test]
    fn ferumbras_short_yes_uses_thirty_essences_and_preserves_topic() -> Result<(), String> {
        let data = dialogue()?;
        let short = data.choose("short", 0, -1).ok_or("short")?;
        assert_eq!(short.next_topic, Some(2));
        let yes = data.choose("yes", 2, -1).ok_or("yes")?;
        assert_eq!(yes.next_topic, None);
        assert!(yes.exchange.is_none());
        assert_eq!(
            yes.reply,
            [
                "Good!, but I need 30 {demonic essences} to exchange with the demonic messenger for a ticket for you to enter the Abodes of Torments."
            ]
        );
        Ok(())
    }

    #[test]
    fn ferumbras_long_story_resets_topic_in_the_pinned_source() -> Result<(), String> {
        let data = dialogue()?;
        let long = data.choose("long", 3, -1).ok_or("long")?;
        assert_eq!(long.next_topic, Some(0));
        assert_eq!(long.reply.len(), 14);
        let yes = data.choose("yes", 0, -1).ok_or("fallback")?;
        assert_eq!(yes.reply, ["What?!"]);
        assert!(yes.exchange.is_none());
        Ok(())
    }

    #[test]
    fn ferumbras_exchange_is_repeatable_and_declares_all_four_writes() -> Result<(), String> {
        let data = dialogue()?;
        for access in [-1, 0, 1, 2] {
            let branch = data.choose("yes", 3, access).ok_or("exchange")?;
            let exchange = branch.exchange.as_ref().ok_or("recipe")?;
            assert_eq!(
                exchange.remove,
                SourceItem {
                    source_id: 6499,
                    count: 30
                }
            );
            assert_eq!(
                exchange.give,
                SourceItem {
                    source_id: 22182,
                    count: 1
                }
            );
            assert_eq!(
                exchange.writes,
                [
                    StorageWrite {
                        storage: 45211,
                        value: 1
                    },
                    StorageWrite {
                        storage: 45216,
                        value: 1
                    },
                    StorageWrite {
                        storage: 45217,
                        value: 1
                    },
                    StorageWrite {
                        storage: 45218,
                        value: 1
                    },
                ]
            );
            assert_eq!(
                exchange.refusal,
                ["You don't have the demonic essences, back here when you get it."]
            );
            assert_eq!(branch.next_topic, Some(0));
        }
        Ok(())
    }

    #[test]
    fn ferumbras_elseif_order_is_not_an_unordered_keyword_map() -> Result<(), String> {
        let data = dialogue()?;
        let accepted = data.choose("yes essence", 2, -1).ok_or("yes")?;
        assert_eq!(accepted.next_topic, None);
        let existing = data.choose("yes essence", 2, 1).ok_or("essence")?;
        assert_eq!(existing.next_topic, Some(3));
        let trade_topic = data.choose("yes essence", 3, 1).ok_or("essence")?;
        assert_eq!(trade_topic.next_topic, Some(3));
        assert!(trade_topic.exchange.is_none());
        Ok(())
    }

    #[test]
    fn ferumbras_keyword_matching_preserves_lua_prefix_and_pattern_rules() {
        assert!(source_keyword_matches("YES", "yes"));
        assert!(source_keyword_matches("yesterday", "yes"));
        assert!(source_keyword_matches("please yes", "yes"));
        assert!(!source_keyword_matches("ayes", "yes"));
        assert!(!source_keyword_matches("yes ayes", "yes"));
        assert!(source_keyword_matches("demi-plane", "demi-plane"));
        assert!(source_keyword_matches("tell me demiplane", "demi-plane"));
        assert!(!source_keyword_matches("tell me demi-plane", "demi-plane"));
        assert!(source_keyword_matches("ab'dendriel", "ab'dendriel"));
        assert!(!source_keyword_matches("cab'dendriel", "ab'dendriel"));
    }

    #[test]
    fn ferumbras_full_source_dialogue_has_every_branch_and_template() -> Result<(), String> {
        let data = dialogue()?;
        assert_eq!(data.branches.len(), 22);
        assert_eq!(
            data.branches
                .iter()
                .map(|b| b.reply.len() + b.exchange.as_ref().map_or(0, |e| e.refusal.len()))
                .sum::<usize>(),
            55
        );
        assert_eq!(
            data.greet,
            "Greetings, dear visitor. Please tell me what {brings} you here, to my humble adobe."
        );
        assert_eq!(data.farewell, "Good bye.");
        assert_eq!(data.walkaway, "Oh well.");
        for word in [
            "brings",
            "problems",
            "godbreaker",
            "ruthless seven",
            "ascension",
            "ferumbras",
            "bozarn",
            "darashia",
            "thais",
            "carlin",
            "kazordoon",
            "ab'dendriel",
            "edron",
            "demons",
            "venore",
        ] {
            let branch = data.choose(word, 0, -1).ok_or("branch")?;
            assert!(!branch.keywords.is_empty(), "{word}");
        }
        Ok(())
    }
    #[test]
    fn ferumbras_first_note_requires_exactly_eight_basins_and_can_reset_note_two()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("ferumbras-tests")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let callbacks = super::super::world_callbacks::load()?;
        let callback = callbacks
            .iter()
            .find(|entry| entry.selector.action == Some(54388))
            .ok_or("note callback")?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        let basins = callback
            .storage
            .iter()
            .find(|s| s.track.ends_with("/basin_counter"))
            .ok_or("basins")?;
        let notes = callback
            .storage
            .iter()
            .find(|s| s.track.ends_with("/tarbaz_notes"))
            .ok_or("notes")?;
        for (basins_before, accepted) in [(7, false), (8, true), (9, false)] {
            let stored = std::collections::BTreeMap::from([
                (basins.track.clone(), basins_before),
                (notes.track.clone(), 2),
            ]);
            let result = catalogue.evaluate(transition, &stored, 0);
            if accepted {
                let changes = result.map_err(|error| format!("{error:?}"))?;
                assert!(changes.iter().any(|change| change.track == notes.track
                    && change.before == 2
                    && change.after == 1));
                assert!(
                    changes
                        .iter()
                        .any(|change| change.track == basins.track && change.after == 8)
                );
            } else {
                assert_eq!(result, Err(super::super::QuestRefusal::StageMismatch));
            }
        }
        Ok(())
    }
}
