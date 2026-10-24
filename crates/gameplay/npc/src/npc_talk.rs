//! NPC talk over the chat path (NPC-TALK-1, CHAT-0 §3, NPC-0 §3.3).
//!
//! A `say` near a runtime NPC is answered by that NPC from catalogue data: a greeting starts a
//! conversation, a topic keyword answers while the speaker stays in range, a farewell ends it.
//! The reply is an ordinary local line spoken by the NPC, so it reaches every session that hears
//! the NPC. Trade and bank are not here (NPC-TRADE-1, BANK-NPC-1). Nothing is durable.

use std::collections::HashMap;

use crate::content::{NpcServiceModel, ProjectV2ServiceOffer};
use crate::foundation::speech::ChatPosition;
use crate::greeting::{DEFAULT_TALK_RANGE, contains_words, words};
use crate::greeting::{GreetingNpc, greeted_npc};

/// Words that start a conversation, until an NPC carries its own.
const GREETINGS: [&str; 3] = ["hi", "hello", "hail"];
/// Words that end a conversation.
const FAREWELLS: [&str; 3] = ["bye", "farewell", "goodbye"];
/// Words that ask what the NPC offers.
const TOPICS: [&str; 4] = ["job", "offer", "trade", "travel"];

/// What one NPC says, from its generated catalogue replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTalkEntry {
    pub name: String,
    bank: bool,
    admitted_dialogue: bool,
    dialogue_keywords: Vec<crate::content::ProjectV2DialogueKeyword>,
    greetings: Vec<String>,
    greeting_reply: String,
    farewell_reply: String,
    /// The service lines, joined; `None` when the NPC has no service.
    topic_reply: Option<String>,
    /// The admitted offers of every Service of the NPC, in source order (NPC-CMD-1b).
    offers: Vec<ProjectV2ServiceOffer>,
    pub premium_services: Vec<crate::content::npc_catalogue::premium_services::PremiumNpcService>,
}

impl NpcTalkEntry {
    pub fn bank(&self) -> bool {
        self.bank
    }

    /// The admitted offers the trade window projects.
    pub fn offers(&self) -> &[ProjectV2ServiceOffer] {
        &self.offers
    }

    /// The line that ends a conversation.
    pub fn farewell(&self) -> &str {
        &self.farewell_reply
    }

    #[doc(hidden)]
    pub fn new(name: &str, topic: Option<&str>) -> Self {
        Self {
            name: name.to_owned(),
            bank: false,
            admitted_dialogue: false,
            dialogue_keywords: Vec::new(),
            greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
            greeting_reply: format!("Greetings, I am {name}."),
            farewell_reply: "Farewell.".to_owned(),
            topic_reply: topic.map(str::to_owned),
            offers: Vec::new(),
            premium_services: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub fn with_bank(mut self) -> Self {
        self.bank = true;
        self
    }

    #[doc(hidden)]
    pub fn with_offers(mut self, offers: Vec<ProjectV2ServiceOffer>) -> Self {
        self.offers = offers;
        self
    }
}

/// The talk of every NPC of the catalogue, by NPC key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcTalkBook {
    npcs: HashMap<String, NpcTalkEntry>,
    /// The revision the trade windows carry: derived from the catalogue's source tree digest, so
    /// it changes exactly when the content the offers come from changes. Never 0.
    catalogue_revision: u64,
}

impl NpcTalkBook {
    /// Entries for every NPC with generated replies. An NPC with an admitted Dialogue is left out
    /// until a dialogue packet reads it.
    pub fn from_model(model: &NpcServiceModel) -> Self {
        let mut book = Self {
            catalogue_revision: revision_of(model.source_tree_digest()),
            ..Self::default()
        };
        for (reference, npc) in model.npcs() {
            let premium_services = crate::content::npc_catalogue::premium_services::resolve(
                &reference.key,
                &npc.services,
            );
            let bank = npc
                .services
                .iter()
                .filter_map(|reference| model.services().get(reference))
                .any(|service| service.bank);
            let (greeting_reply, farewell_reply, topic_reply, dialogue_keywords) =
                if let Some(replies) = &npc.generated_replies {
                    let mut lines: Vec<&str> = Vec::new();
                    for (_, line) in &replies.services {
                        if !lines.contains(&line.text.as_str()) {
                            lines.push(&line.text);
                        }
                    }
                    (
                        replies.greeting.text.clone(),
                        replies.farewell.text.clone(),
                        (!lines.is_empty()).then(|| lines.join(" ")),
                        Vec::new(),
                    )
                } else if !premium_services.is_empty() {
                    (
                        format!("Greetings, I am {}.", npc.name),
                        "Farewell.".into(),
                        Some("Ask me about promotion or a blessing.".into()),
                        Vec::new(),
                    )
                } else if bank {
                    // Only the Bank Dialogue's explicitly supported, authored message shape is
                    // consumed. Missing/multipart/stateful dialogue stays held, never generated.
                    let Some(dialogue) = &npc.bank_dialogue else {
                        continue;
                    };
                    if dialogue.greet.len() != 1
                        || dialogue.farewell.len() != 1
                        || dialogue.keywords.iter().any(|keyword| {
                            keyword.reply.len() != 1
                                || !keyword.children.is_empty()
                                || keyword.fallback
                                || keyword.only_unfocus
                                || keyword.reset
                                || keyword.ungreet
                                || keyword.move_up.is_some()
                                || !keyword.links.is_empty()
                        })
                    {
                        continue;
                    }
                    (
                        dialogue.greet[0].clone(),
                        dialogue.farewell[0].clone(),
                        None,
                        dialogue.keywords.clone(),
                    )
                } else {
                    continue;
                };
            book.npcs.insert(
                reference.key.clone(),
                NpcTalkEntry {
                    name: npc.name.clone(),
                    admitted_dialogue: npc.dialogue.is_some() && premium_services.is_empty(),
                    premium_services,
                    bank: npc
                        .services
                        .iter()
                        .filter_map(|reference| model.services().get(reference))
                        .any(|service| service.bank),
                    greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
                    greeting_reply,
                    farewell_reply,
                    topic_reply,
                    dialogue_keywords,
                    offers: npc
                        .services
                        .iter()
                        .filter_map(|service| model.services().get(service))
                        .flat_map(|service| service.offers.iter().cloned())
                        .collect(),
                },
            );
        }
        book
    }

    #[doc(hidden)]
    pub fn with(mut self, key: &str, entry: NpcTalkEntry) -> Self {
        self.npcs.insert(key.to_owned(), entry);
        self
    }

    pub fn get(&self, key: &str) -> Option<&NpcTalkEntry> {
        self.npcs.get(key)
    }

    pub fn catalogue_revision(&self) -> u64 {
        self.catalogue_revision.max(1)
    }
}

/// FNV-1a over the digest text; 1 in place of 0.
fn revision_of(digest: &str) -> u64 {
    let hash = digest
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    hash.max(1)
}

/// An NPC the speaker could talk to: where it stands and what it says.
pub struct TalkNpc<'a, K> {
    pub id: K,
    pub position: ChatPosition,
    pub entry: &'a NpcTalkEntry,
}

/// The NPC's reply to a `say`: the NPC that answers and its text.
pub struct NpcReply<K> {
    pub id: K,
    pub text: String,
}

/// Whether `text` asks the NPC to trade: the topic that opens the trade window.
pub fn asks_to_trade(text: &str) -> bool {
    contains_words(&words(text), &["trade".to_owned()])
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
pub fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &str,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    let said = words(text);
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
        if let Some(keyword) = npc
            .entry
            .dialogue_keywords
            .iter()
            .find(|keyword| contains_words(&said, &keyword.triggers))
        {
            return Some(NpcReply {
                id: npc.id,
                text: keyword.reply[0].clone(),
            });
        }
        if says_any(&said, &TOPICS) && !npc.entry.admitted_dialogue {
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
    text: &str,
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

    fn say(text: &str) -> String {
        text.to_owned()
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod bank_dialogue_tests {
    use super::*;
    #[test]
    fn admitted_bank_dialogue_never_generates_an_unmatched_topic_answer() {
        let mut entry = NpcTalkEntry::new("Paulie", None);
        entry.bank = true;
        entry.admitted_dialogue = true;
        entry.greeting_reply = "Welcome to the bank.".into();
        entry.farewell_reply = "Farewell from the admitted Dialogue.".into();
        let position = ChatPosition {
            x: 1,
            y: 1,
            floor: 0,
        };
        let npcs = [TalkNpc {
            id: 1_u8,
            position,
            entry: &entry,
        }];
        let mut engaged = None;
        let greet = "hi";
        assert_eq!(
            talk(position, greet, &npcs, &mut engaged).map(|r| r.text),
            Some("Welcome to the bank.".into())
        );
        for topic in ["job", "offer", "trade", "travel"] {
            assert!(talk(position, topic, &npcs, &mut engaged).is_none());
            assert_eq!(engaged, Some(1));
        }
        let farewell = talk(position, "bye", &npcs, &mut engaged);
        assert_eq!(
            farewell.map(|r| r.text),
            Some("Farewell from the admitted Dialogue.".into())
        );
        assert_eq!(engaged, None);
    }
}
