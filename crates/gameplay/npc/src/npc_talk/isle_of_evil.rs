//! Exact source message admission for the five existing Isle dialogue NPCs.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TEXT: &str = include_str!("../../../../../content/dialogues/quests/isle-of-evil.json");
const KEYS: [&str; 5] = [
    "oteryn:npc.rapanaio",
    "oteryn:npc.rapanaio_boat",
    "oteryn:npc.rapanaio_isle_of_evil",
    "oteryn:npc.scutty",
    "oteryn:npc.mortimer",
];
#[derive(Deserialize)]
struct Document {
    schema: String,
    source_revision: String,
    npcs: Vec<Npc>,
    replies: BTreeMap<String, Reply>,
}
#[derive(Deserialize)]
struct Npc {
    key: String,
    greet: String,
    farewell: String,
    walkaway: String,
}
#[derive(Deserialize)]
struct Reply {
    lines: Vec<String>,
    source: String,
}

pub(super) struct Messages {
    pub(super) greet: String,
    pub(super) farewell: String,
    pub(super) walkaway: String,
}

fn data() -> Option<&'static Document> {
    static DATA: OnceLock<Option<Document>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Document = serde_json::from_str(TEXT).ok()?;
        (data.schema == "OTERYN_ISLE_OF_EVIL_DIALOGUE_V1"
            && data.source_revision == "201189f871d23597b82caf88fd611454a2ddefb8"
            && data.npcs.len() == KEYS.len()
            && data.npcs.iter().zip(KEYS).all(|(npc, key)| npc.key == key))
        .then_some(data)
    })
    .as_ref()
}

pub(super) fn messages(key: &str) -> Option<Messages> {
    if !KEYS.contains(&key) {
        return None;
    }
    let data = data()?;
    let npc = data.npcs.iter().find(|npc| npc.key == key)?;
    let line = |key: &String| {
        let reply = data.replies.get(key)?;
        if reply.lines.len() != 1
            || !reply
                .source
                .starts_with("zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:")
        {
            return None;
        }
        reply.lines.first().cloned()
    };
    Some(Messages {
        greet: line(&npc.greet)?,
        farewell: line(&npc.farewell)?,
        walkaway: line(&npc.walkaway)?,
    })
}

pub(super) fn revision_material() -> &'static str {
    TEXT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc_talk::{NpcTalkBook, TalkNpc, talk};

    #[test]
    fn isle_real_npc_book_opens_five_conversations_and_preserves_main_services()
    -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = NpcTalkBook::from_model(&model);
        let at = |x| crate::foundation::speech::ChatPosition {
            x,
            y: 100,
            floor: 7,
        };
        for key in KEYS {
            let expected = messages(key).ok_or("source text")?;
            let entry = book
                .get(key)
                .ok_or_else(|| format!("missing actual NPC entry {key}"))?;
            let (_, npc) = model
                .npcs()
                .iter()
                .find(|(reference, _)| reference.key == key)
                .ok_or("source NPC")?;
            if let Some(replies) = &npc.generated_replies {
                assert_eq!(entry.greeting_reply, replies.greeting.text);
                assert_eq!(entry.farewell_reply, replies.farewell.text);
            } else if !entry.travel_routes.is_empty() || !entry.premium_services.is_empty() {
                // These services already opened on main; adding a quest caller
                // must preserve their greeting and service topic.
                assert_eq!(
                    entry.greeting_reply,
                    format!("Greetings, I am {}.", npc.name)
                );
                assert_eq!(entry.farewell_reply, "Farewell.");
                assert!(entry.topic_reply.is_some());
            } else {
                assert_eq!(entry.greeting_reply, expected.greet);
                assert_eq!(entry.farewell_reply, expected.farewell);
                assert!(entry.topic_reply.is_none());
            }
            assert_eq!(
                entry.walkaway_for("Explorer"),
                expected.walkaway.replace("|PLAYERNAME|", "Explorer")
            );
            assert_eq!(
                entry.render_source_text(entry.farewell(), "Explorer"),
                entry.farewell().replace("|PLAYERNAME|", "Explorer")
            );
            assert!(entry.dialogue_keywords.is_empty());
            assert!(!entry.bank());
            let npcs = [TalkNpc {
                id: 1_u8,
                position: at(101),
                entry,
            }];
            let mut engaged = None;
            assert!(talk(at(100), "hi", &npcs, &mut engaged).is_some());
            assert_eq!(engaged, Some(1));
            assert!(talk(at(100), "yes", &npcs, &mut engaged).is_none());
        }
        Ok(())
    }

    #[test]
    fn isle_does_not_admit_the_kings_foreign_greeting_or_unknown_keys() {
        for key in ["oteryn:npc.king_tibianus", "oteryn:npc.rapanaio_fake", ""] {
            assert!(messages(key).is_none());
        }
    }
}
