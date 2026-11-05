//! Targuna conversation admission. Source greetings open only the six existing NPCs;
//! state-dependent greetings and every quest effect remain in the fenced dispatch caller.
//! Text comes from Crystal 201189f871d23597b82caf88fd611454a2ddefb8, never generated lore.

use std::sync::OnceLock;

use serde::Deserialize;

const TEXT: &str = include_str!("../../../../../content/dialogues/targuna.json");
const NAMES: [&str; 6] = [
    "lizzie", "camilla", "sterling", "morla", "emiliana", "adrian",
];

#[derive(Deserialize)]
struct Document {
    schema: String,
    source_only: bool,
    records: Vec<NpcText>,
}

#[derive(Deserialize)]
struct NpcText {
    npc: String,
    source: Source,
    text_records: Vec<Message>,
}

#[derive(Deserialize)]
struct Source {
    repository: String,
    commit: String,
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Message {
    kind: String,
    event: Option<String>,
    parts: Vec<String>,
}

fn data() -> Option<&'static Document> {
    static DATA: OnceLock<Option<Document>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Document = serde_json::from_str(TEXT).ok()?;
        (data.schema == "oteryn.quests1.targuna-dialogue-source.v1"
            && !data.source_only
            && data.records.len() == NAMES.len()
            && data.records.iter().zip(NAMES).all(|(npc, name)| {
                npc.npc == name
                    && npc.source.repository == "zimbadev/crystalserver"
                    && npc.source.commit == "201189f871d23597b82caf88fd611454a2ddefb8"
                    && npc.source.path == format!("data-global/npc/{name}.lua")
                    && npc.source.sha256.len() == 64
            }))
        .then_some(data)
    })
    .as_ref()
}

fn message(npc: &NpcText, event: &str) -> Option<String> {
    // Final setMessage installs the NPC's initial message outside the stateful callbacks.
    // Dynamic greet callbacks are selected later by dispatch after reading QuestState.
    let message = npc
        .text_records
        .iter()
        .rev()
        .find(|message| message.kind == "message" && message.event.as_deref() == Some(event))?;
    if message.parts.len() != 1 {
        return None;
    }
    message.parts.first().cloned()
}

pub(super) fn greeting(key: &str) -> Option<(String, String)> {
    let name = key.strip_prefix("oteryn:npc.")?;
    if !NAMES.contains(&name) {
        return None;
    }
    let npc = data()?.records.iter().find(|npc| npc.npc == name)?;
    Some((
        message(npc, "MESSAGE_GREET")?,
        message(npc, "MESSAGE_FAREWELL")?,
    ))
}

/// Include the scoped text document in the conversation generation's catalogue revision.
pub(super) fn revision_material() -> &'static str {
    TEXT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc_talk::{NpcTalkBook, TalkNpc, talk};

    #[test]
    fn targuna_source_greeting_is_an_exact_six_npc_scope() {
        for key in [
            "oteryn:npc.lizzie_extra",
            "oteryn:npc.emiliana_fake",
            "oteryn:npc.bozo",
            "",
        ] {
            assert!(greeting(key).is_none());
        }
        assert_eq!(
            greeting("oteryn:npc.lizzie"),
            Some((
                "Hey there. I need your help urgently, we have to {save} Targuna!".into(),
                "Goodbye!".into(),
            ))
        );
        assert_eq!(
            greeting("oteryn:npc.emiliana"),
            Some((
                "Hey, I really need your {help} on an important mission urgently.".into(),
                "Good bye!".into(),
            ))
        );
    }

    #[test]
    fn targuna_six_existing_npcs_open_only_source_conversations() -> Result<(), String> {
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
        for name in NAMES {
            let key = format!("oteryn:npc.{name}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("missing source conversation: {key}"))?;
            let (greet, farewell) =
                greeting(&key).ok_or_else(|| format!("missing source text: {key}"))?;
            assert_eq!(entry.greeting_reply, greet);
            assert_eq!(entry.farewell_reply, farewell);
            assert!(entry.dialogue_keywords.is_empty());
            assert!(entry.topic_reply.is_none());
            assert!(
                !entry.bank(),
                "no bank service is added by a quest greeting"
            );
            assert!(entry.offers().is_empty());
            let npcs = [TalkNpc {
                id: 1_u8,
                position: at(101),
                entry,
            }];
            let mut engaged = None;
            assert!(talk(at(100), "hi", &npcs, &mut engaged).is_some());
            assert_eq!(engaged, Some(1));
            assert!(
                talk(at(100), "yes", &npcs, &mut engaged).is_none(),
                "opening a conversation cannot grant a source quest branch"
            );
            assert_eq!(engaged, Some(1));
        }
        Ok(())
    }
}
