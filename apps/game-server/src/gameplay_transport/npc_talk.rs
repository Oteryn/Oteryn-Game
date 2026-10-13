//! NPC talk over the chat path (NPC-TALK-1, CHAT-0 §3, NPC-0 §3.3).
//!
//! A `say` near a runtime NPC is answered by that NPC from catalogue data: a greeting starts a
//! conversation, a topic keyword answers while the speaker stays in range, a farewell ends it.
//! The reply is an ordinary local line spoken by the NPC, so it reaches every session that hears
//! the NPC. Trade and bank are not here (NPC-TRADE-1, BANK-NPC-1). Nothing is durable.

use std::collections::HashMap;

use crate::chat::greeting::{DEFAULT_TALK_RANGE, contains_words, words};
use crate::chat::{ChatPosition, ChatText, GreetingNpc, greeted_npc};
use crate::content::NpcServiceModel;

/// Words that start a conversation, until an NPC carries its own.
const GREETINGS: [&str; 3] = ["hi", "hello", "hail"];
/// Words that end a conversation.
const FAREWELLS: [&str; 3] = ["bye", "farewell", "goodbye"];
/// Words that ask what the NPC offers.
const TOPICS: [&str; 4] = ["job", "offer", "trade", "travel"];

/// What one NPC says, from its generated catalogue replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NpcTalkEntry {
    pub(crate) name: String,
    greetings: Vec<String>,
    greeting_reply: String,
    farewell_reply: String,
    /// The service lines, joined; `None` when the NPC has no service.
    topic_reply: Option<String>,
}

impl NpcTalkEntry {
    /// The line that ends a conversation.
    pub(crate) fn farewell(&self) -> &str {
        &self.farewell_reply
    }

    #[cfg(test)]
    pub(crate) fn new(name: &str, topic: Option<&str>) -> Self {
        Self {
            name: name.to_owned(),
            greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
            greeting_reply: format!("Greetings, I am {name}."),
            farewell_reply: "Farewell.".to_owned(),
            topic_reply: topic.map(str::to_owned),
        }
    }
}

/// The talk of every NPC of the catalogue, by NPC key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NpcTalkBook {
    npcs: HashMap<String, NpcTalkEntry>,
}

impl NpcTalkBook {
    /// Entries for every NPC with generated replies. An NPC with an admitted Dialogue is left out
    /// until a dialogue packet reads it.
    pub(crate) fn from_model(model: &NpcServiceModel) -> Self {
        let mut book = Self::default();
        for (reference, npc) in model.npcs() {
            let Some(replies) = &npc.generated_replies else {
                continue;
            };
            let mut lines: Vec<&str> = Vec::new();
            for (_, line) in &replies.services {
                if !lines.contains(&line.text.as_str()) {
                    lines.push(&line.text);
                }
            }
            book.npcs.insert(
                reference.key.clone(),
                NpcTalkEntry {
                    name: npc.name.clone(),
                    greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
                    greeting_reply: replies.greeting.text.clone(),
                    farewell_reply: replies.farewell.text.clone(),
                    topic_reply: (!lines.is_empty()).then(|| lines.join(" ")),
                },
            );
        }
        book
    }

    #[cfg(test)]
    pub(crate) fn with(mut self, key: &str, entry: NpcTalkEntry) -> Self {
        self.npcs.insert(key.to_owned(), entry);
        self
    }

    pub(crate) fn get(&self, key: &str) -> Option<&NpcTalkEntry> {
        self.npcs.get(key)
    }
}

/// An NPC the speaker could talk to: where it stands and what it says.
pub(crate) struct TalkNpc<'a, K> {
    pub(crate) id: K,
    pub(crate) position: ChatPosition,
    pub(crate) entry: &'a NpcTalkEntry,
}

/// The NPC's reply to a `say`: the NPC that answers and its text.
pub(crate) struct NpcReply<K> {
    pub(crate) id: K,
    pub(crate) text: String,
}

fn says_any(said: &[String], list: &[&str]) -> bool {
    list.iter()
        .any(|word| contains_words(said, &[(*word).to_owned()]))
}

/// Applies `text`, said at `speaker`, to the NPC conversation `engaged`.
///
/// A conversation ends when its NPC is gone or out of range. While it runs, a farewell ends it
/// and a topic is answered; otherwise a greeting within range starts one (the nearest NPC, then
/// the lowest index in `npcs`).
pub(crate) fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    let said = words(text.as_str());
    if let Some(current) = *engaged {
        let here = npcs.iter().find(|npc| {
            npc.id == current
                && speaker
                    .distance(npc.position)
                    .is_some_and(|d| d <= DEFAULT_TALK_RANGE)
        });
        let Some(npc) = here else {
            *engaged = None;
            return start(speaker, text, npcs, engaged);
        };
        if says_any(&said, &FAREWELLS) {
            *engaged = None;
            return Some(NpcReply {
                id: npc.id,
                text: npc.entry.farewell_reply.clone(),
            });
        }
        if says_any(&said, &TOPICS) {
            let line =
                npc.entry.topic_reply.clone().unwrap_or_else(|| {
                    format!("{} has nothing to offer right now.", npc.entry.name)
                });
            return Some(NpcReply {
                id: npc.id,
                text: line,
            });
        }
    }
    start(speaker, text, npcs, engaged)
}

fn start<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    let greeting: Vec<GreetingNpc<'_>> = npcs
        .iter()
        .enumerate()
        .map(|(index, npc)| GreetingNpc {
            actor_id: index as u64,
            position: npc.position,
            greetings: &npc.entry.greetings,
            talk_range: DEFAULT_TALK_RANGE,
        })
        .collect();
    let index = usize::try_from(greeted_npc(speaker, text, &greeting)?).ok()?;
    let npc = npcs.get(index)?;
    *engaged = Some(npc.id);
    Some(NpcReply {
        id: npc.id,
        text: npc.entry.greeting_reply.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;

    fn entry(name: &str, topic: Option<&str>) -> NpcTalkEntry {
        NpcTalkEntry::new(name, topic)
    }

    fn at(x: i32) -> ChatPosition {
        ChatPosition {
            x,
            y: 100,
            floor: 7,
        }
    }

    fn say(text: &str) -> ChatText {
        ChatText::parse(text).expect("text")
    }

    #[test]
    fn a_conversation_runs_greeting_topic_farewell() {
        let alice = entry("Alice", Some("Alice buys and sells goods."));
        let npcs = [TalkNpc {
            id: 1_u8,
            position: at(102),
            entry: &alice,
        }];
        let mut engaged = None;
        // A topic before a greeting is not answered.
        assert!(talk(at(100), &say("trade"), &npcs, &mut engaged).is_none());
        let reply = talk(at(100), &say("Hello!"), &npcs, &mut engaged).expect("greeted");
        assert_eq!(reply.text, "Greetings, I am Alice.");
        assert_eq!(engaged, Some(1));
        let reply = talk(at(100), &say("what is your job"), &npcs, &mut engaged).expect("topic");
        assert_eq!(reply.text, "Alice buys and sells goods.");
        let reply = talk(at(100), &say("bye"), &npcs, &mut engaged).expect("farewell");
        assert_eq!(reply.text, "Farewell.");
        assert_eq!(engaged, None);
        assert!(talk(at(100), &say("trade"), &npcs, &mut engaged).is_none());
    }

    #[test]
    fn walking_out_of_range_ends_the_conversation() {
        let alice = entry("Alice", None);
        let npcs = [TalkNpc {
            id: 1_u8,
            position: at(102),
            entry: &alice,
        }];
        let mut engaged = Some(1);
        assert!(talk(at(110), &say("trade"), &npcs, &mut engaged).is_none());
        assert_eq!(engaged, None);
        assert!(talk(at(110), &say("hi"), &npcs, &mut engaged).is_none());
    }

    #[test]
    fn an_npc_without_a_service_says_so() {
        let bob = entry("Bob", None);
        let npcs = [TalkNpc {
            id: 2_u8,
            position: at(101),
            entry: &bob,
        }];
        let mut engaged = None;
        talk(at(100), &say("hi"), &npcs, &mut engaged).expect("greeted");
        let reply = talk(at(100), &say("offer"), &npcs, &mut engaged).expect("topic");
        assert_eq!(reply.text, "Bob has nothing to offer right now.");
    }

    #[test]
    fn the_nearest_greeted_npc_answers() {
        let (far, near) = (entry("Far", None), entry("Near", None));
        let npcs = [
            TalkNpc {
                id: 1_u8,
                position: at(103),
                entry: &far,
            },
            TalkNpc {
                id: 2_u8,
                position: at(101),
                entry: &near,
            },
        ];
        let mut engaged = None;
        let reply = talk(at(100), &say("hi"), &npcs, &mut engaged).expect("greeted");
        assert_eq!((reply.id, engaged), (2, Some(2)));
    }
}
