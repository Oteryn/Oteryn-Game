//! Complete source text; dynamic %d values are supplied only by the closed NPC rule.
//! No Lua interpreter or user-controlled format string is introduced.
use std::sync::OnceLock;

use serde::Deserialize;

use super::npc_rules::{Npc, Reply};

const EMBEDDED: &str = include_str!("../../../../../../content/dialogues/targuna.json");

#[derive(Deserialize)]
struct Document {
    schema: String,
    npc_count: usize,
    text_count: usize,
    records: Vec<NpcText>,
}
#[derive(Deserialize)]
struct NpcText {
    npc: String,
    source: Source,
    text_records: Vec<Text>,
}
#[derive(Deserialize)]
struct Source {
    repository: String,
    commit: String,
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Text {
    source_line: u16,
    kind: String,
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    parts: Vec<String>,
    format: Option<String>,
    #[serde(default)]
    arguments: Vec<String>,
}

fn data() -> Option<&'static Document> {
    static DATA: OnceLock<Option<Document>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Document = serde_json::from_str(EMBEDDED).ok()?;
        (data.schema == "oteryn.quests1.targuna-dialogue-source.v1"
            && data.records.len() == data.npc_count
            && data.npc_count == 6
            && data
                .records
                .iter()
                .map(|npc| npc.text_records.len())
                .sum::<usize>()
                == data.text_count
            && data.records.iter().all(|npc| {
                npc.source.repository == "zimbadev/crystalserver"
                    && npc.source.commit == "201189f871d23597b82caf88fd611454a2ddefb8"
                    && npc.source.path == format!("data-global/npc/{}.lua", npc.npc)
                    && npc.source.sha256.len() == 64
            }))
        .then_some(data)
    })
    .as_ref()
}

pub fn render(npc: Npc, reply: &Reply) -> Option<Vec<String>> {
    let record = data()?
        .records
        .iter()
        .find(|record| record.npc == npc.source_name())?;
    let text = record
        .text_records
        .iter()
        .find(|text| text.source_line == reply.source_line)?;
    match text.format.as_deref() {
        None if reply.numbers.is_empty() => Some(text.parts.clone()),
        Some("lua_string_format") if text.arguments.len() == reply.numbers.len() => {
            let mut values = reply.numbers.iter();
            let mut result = Vec::new();
            for part in &text.parts {
                let mut line = String::new();
                let mut rest = part.as_str();
                while let Some((before, after)) = rest.split_once("%d") {
                    line.push_str(before);
                    line.push_str(&values.next()?.to_string());
                    rest = after;
                }
                line.push_str(rest);
                result.push(line);
            }
            values.next().is_none().then_some(result)
        }
        _ => None,
    }
}

/// The native talk owner emits the last source MESSAGE_GREET on opening. A first
/// command after an earlier public greeting has no such line and must execute its
/// actual mission keyword immediately, rather than greeting a second time.
pub fn is_opening_greeting(npc: Npc, lines: &[String]) -> bool {
    data()
        .and_then(|data| {
            data.records
                .iter()
                .find(|record| record.npc == npc.source_name())
        })
        .and_then(|record| {
            record.text_records.iter().rev().find(|text| {
                text.kind == "message" && text.event.as_deref() == Some("MESSAGE_GREET")
            })
        })
        .is_some_and(|text| text.parts == lines)
}

/// Static KeywordHandler rules precede CALLBACK_MESSAGE_DEFAULT in the source handler.
/// Crystal data/npclib/npc_system/keyword_handler.lua:57-68 requires every
/// keyword's first occurrence in non-decreasing position; lists are not alternatives.
pub fn keyword(npc: Npc, message: &str) -> Option<Reply> {
    let record = data()?
        .records
        .iter()
        .find(|record| record.npc == npc.source_name())?;
    let text = record
        .text_records
        .iter()
        .find(|text| text.kind == "keyword" && keyword_list_matches(message, &text.keywords))?;
    Some(Reply {
        source_line: text.source_line,
        numbers: Vec::new(),
    })
}

fn keyword_list_matches(message: &str, keywords: &[String]) -> bool {
    let message = message.to_ascii_lowercase();
    let mut previous = 0;
    !keywords.is_empty()
        && keywords.iter().all(|keyword| {
            // The captured six-NPC keyword corpus contains only literal ASCII words/spaces.
            if !keyword
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b' ')
            {
                return false;
            }
            let Some(index) = message.find(keyword) else {
                return false;
            };
            if index < previous {
                return false;
            }
            previous = index;
            true
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targuna_only_the_ordinary_opening_line_triggers_a_dynamic_greeting() -> Result<(), String> {
        for npc in [
            Npc::Lizzie,
            Npc::Camilla,
            Npc::Sterling,
            Npc::Morla,
            Npc::Emiliana,
            Npc::Adrian,
        ] {
            let initial = render(
                npc,
                &Reply {
                    source_line: super::super::npc_rules::greet(
                        npc,
                        Default::default(),
                        super::super::npc_rules::EmilianaPlace::Targuna,
                    ),
                    numbers: Vec::new(),
                },
            )
            .ok_or("source opening")?;
            assert!(is_opening_greeting(npc, &initial));
            assert!(!is_opening_greeting(npc, &[]));
        }
        let report = render(
            Npc::Lizzie,
            &Reply {
                source_line: 93,
                numbers: Vec::new(),
            },
        )
        .ok_or("source mission")?;
        assert!(!is_opening_greeting(Npc::Lizzie, &report));
        Ok(())
    }

    #[test]
    fn targuna_full_dialogue_keeps_multipart_order_and_dynamic_deficits() -> Result<(), String> {
        let parts = render(
            Npc::Lizzie,
            &Reply {
                source_line: 93,
                numbers: vec![],
            },
        )
        .ok_or("source reply")?;
        assert_eq!(parts.len(), 2);
        assert!(parts[0].contains("ancient lizard clan"));
        assert!(parts[1].contains("Camilla"));
        let parts = render(
            Npc::Morla,
            &Reply {
                source_line: 102,
                numbers: vec![3, 2],
            },
        )
        .ok_or("source format")?;
        assert_eq!(
            parts,
            vec![
                "Not quite yet! You still need to defeat 3 more pirates and rescue 2 more eggs around the island."
            ]
        );
        assert!(
            render(
                Npc::Morla,
                &Reply {
                    source_line: 102,
                    numbers: vec![3]
                }
            )
            .is_none()
        );
        assert!(
            render(
                Npc::Lizzie,
                &Reply {
                    source_line: 93,
                    numbers: vec![1]
                }
            )
            .is_none()
        );
        Ok(())
    }

    #[test]
    fn targuna_all_six_source_npcs_keep_every_captured_line() -> Result<(), String> {
        let data = data().ok_or("pinned complete dialogue")?;
        assert_eq!(data.text_count, 154);
        for npc in [
            Npc::Lizzie,
            Npc::Camilla,
            Npc::Sterling,
            Npc::Morla,
            Npc::Emiliana,
            Npc::Adrian,
        ] {
            let record = data
                .records
                .iter()
                .find(|record| record.npc == npc.source_name())
                .ok_or("NPC")?;
            assert!(
                record
                    .text_records
                    .iter()
                    .all(|text| !text.parts.is_empty())
            );
        }
        assert!(keyword(Npc::Adrian, "functions").is_some());
        Ok(())
    }

    #[test]
    fn targuna_static_keyword_lists_are_ordered_conjunctions() {
        let words = vec!["sell".into(), "buy".into()];
        assert!(!keyword_list_matches("sell", &words));
        assert!(!keyword_list_matches("buy then sell", &words));
        assert!(keyword_list_matches("sell then buy", &words));
        assert!(keyword_list_matches(
            "pirates",
            &["pirates".into(), "pirate".into()]
        ));
        assert!(keyword_list_matches("myjob", &["job".into()]));
    }
}
