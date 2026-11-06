//! Literal voices for NPCs whose state branches are executed by native quest dispatch.
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::sync::LazyLock;

pub(super) const TEXT: &str =
    include_str!("../../../../../content/dialogues/quests/npc-state-stories.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    #[serde(rename = "default_messages_source")]
    _default_messages_source: IgnoredAny,
    npcs: Vec<Messages>,
    #[serde(rename = "branches")]
    _branches: IgnoredAny,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Messages {
    npc: String,
    pub greet: String,
    pub farewell: String,
    pub walkaway: String,
    #[serde(rename = "source")]
    _source: IgnoredAny,
}

pub(super) fn messages(npc: &str) -> Option<Messages> {
    static DATA: LazyLock<Option<Document>> = LazyLock::new(|| {
        let data: Document = serde_json::from_str(TEXT).ok()?;
        let mut seen = std::collections::BTreeSet::new();
        (data.schema == 1
            && data.npcs.len() <= 64
            && data.npcs.iter().all(|message| {
                message.npc.starts_with("oteryn:npc.")
                    && seen.insert(&message.npc)
                    && [&message.greet, &message.farewell, &message.walkaway]
                        .into_iter()
                        .all(|line| {
                            !line.is_empty()
                                && line.len() <= 1024
                                && !line.chars().any(char::is_control)
                        })
            }))
        .then_some(data)
    });
    DATA.as_ref()?
        .npcs
        .iter()
        .find(|entry| entry.npc == npc)
        .cloned()
}
