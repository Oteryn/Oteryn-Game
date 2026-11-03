//! Literal conversation admission for this batch's five source-qualified NPCs.
//! Stateful quest selection remains with the fenced dispatch caller.
use serde::Deserialize;
use std::sync::OnceLock;

const TEXT: &str = include_str!("../../../../../content/dialogues/quests/batch-one-greetings.json");
const KEYS: [&str; 5] = [
    "oteryn:npc.ezebeth",
    "oteryn:npc.gnomilly",
    "oteryn:npc.gnombold",
    "oteryn:npc.gnomargery",
    "oteryn:npc.mazarius",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    records: Vec<Messages>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Messages {
    npc: String,
    pub(super) greet: Line,
    pub(super) farewell: Line,
    pub(super) walkaway: Line,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Line {
    pub(super) text: String,
    source: String,
}

fn data() -> Option<&'static Document> {
    static DATA: OnceLock<Option<Document>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Document = serde_json::from_str(TEXT).ok()?;
        (data.schema == "OTERYN_QUESTS1_GREETINGS_V1"
            && data.records.len() == KEYS.len()
            && data.records.iter().zip(KEYS).all(|(record, key)| {
                record.npc == key
                    && [&record.greet, &record.farewell, &record.walkaway]
                        .into_iter()
                        .all(|line| {
                            !line.text.is_empty() && line.source.starts_with(
                        "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:")
                        })
            }))
        .then_some(data)
    })
    .as_ref()
}

pub(super) fn messages(key: &str) -> Option<&'static Messages> {
    if !KEYS.contains(&key) {
        return None;
    }
    data()?.records.iter().find(|record| record.npc == key)
}

pub(super) fn revision_material() -> &'static str {
    TEXT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc_talk::{NpcTalkBook, TalkNpc, talk};

    fn book() -> Result<NpcTalkBook, String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        Ok(NpcTalkBook::from_model(&model))
    }

    #[test]
    fn five_source_npcs_open_real_conversations_without_admitting_quest_effects()
    -> Result<(), String> {
        let book = book()?;
        let at = |x| crate::foundation::speech::ChatPosition {
            x,
            y: 100,
            floor: 7,
        };
        for key in KEYS {
            let expected = messages(key).ok_or("source text")?;
            let entry = book
                .get(key)
                .ok_or_else(|| format!("missing real NPC conversation {key}"))?;
            assert_eq!(entry.greeting_reply, expected.greet.text);
            assert_eq!(entry.farewell_reply, expected.farewell.text);
            assert!(entry.dialogue_keywords.is_empty());
            assert!(entry.topic_reply.is_none());
            let npcs = [TalkNpc {
                id: 1_u8,
                position: at(101),
                entry,
            }];
            let mut engaged = None;
            assert_eq!(
                talk(at(100), "hi", &npcs, &mut engaged).map(|reply| reply.text),
                Some(expected.greet.text.clone())
            );
            assert_eq!(engaged, Some(1));
            assert!(talk(at(100), "yes", &npcs, &mut engaged).is_none());
            assert_eq!(engaged, Some(1));
        }
        Ok(())
    }

    #[test]
    fn source_playername_is_literal_data_and_is_substituted_exactly_once() -> Result<(), String> {
        let book = book()?;
        let entry = book.get("oteryn:npc.ezebeth").ok_or("Ezebeth")?;
        let name = "[|PLAYERNAME|] ${name} %s {quest}";
        assert_eq!(
            entry.render_source_text(entry.farewell(), name),
            "Good bye, [|PLAYERNAME|] ${name} %s {quest}."
        );
        assert_eq!(entry.walkaway_for(name), "Good bye.");
        let ordinary = crate::npc_talk::NpcTalkEntry::new("Other", None);
        assert_eq!(
            ordinary.render_source_text("Hello, |PLAYERNAME|.", name),
            "Hello, |PLAYERNAME|."
        );
        Ok(())
    }

    #[test]
    fn unknown_and_sex_guarded_greetings_stay_outside_this_cohort() {
        for key in [
            "oteryn:npc.bozo",
            "oteryn:npc.ezebeth_extra",
            "oteryn:npc.a_beggar",
            "",
        ] {
            assert!(messages(key).is_none());
        }
    }
}
