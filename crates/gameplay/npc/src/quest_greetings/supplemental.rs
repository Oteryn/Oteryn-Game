//! Reusable closed admission for additional static NPC greetings. The caller's
//! immutable bindings preserve identity, exact source custody and literal text.
use super::{Document, Greeting, PIN, Source};
use std::collections::BTreeSet;
const HANDLER: &str = "data/npclib/npc_system/npc_handler.lua";
const HANDLER_SHA: &str = "8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a";
pub(super) struct Binding {
    pub npc: &'static str,
    pub path: &'static str,
    pub sha256: &'static str,
    pub line: u32,
    pub greet: &'static str,
}
fn source(value: &Source, path: &str, sha: &str, line: u32) -> bool {
    value.repository == "zimbadev/crystalserver"
        && value.revision == PIN
        && value.path == path
        && value.sha256 == sha
        && value.line == line
}
pub(super) fn parse(text: &str, bindings: &[Binding]) -> Option<Vec<Greeting>> {
    let data: Document = serde_json::from_str(text).ok()?;
    if bindings.is_empty()
        || bindings.len() > 64
        || data.schema != 1
        || data.source_revision != PIN
        || data.npcs.len() != bindings.len()
        || !source(&data.placeholder_source, HANDLER, HANDLER_SHA, 383)
    {
        return None;
    }
    let mut seen = BTreeSet::new();
    for entry in &data.npcs {
        let binding = bindings.iter().find(|binding| binding.npc == entry.npc)?;
        if !seen.insert(&entry.npc)
            || !entry.variants.is_empty()
            || entry.greet.text != binding.greet
            || !source(
                &entry.greet.source,
                binding.path,
                binding.sha256,
                binding.line,
            )
            || !source(&entry.farewell.source, HANDLER, HANDLER_SHA, 82)
            || !source(&entry.walkaway.source, HANDLER, HANDLER_SHA, 90)
            || [&entry.greet, &entry.farewell, &entry.walkaway]
                .into_iter()
                .any(|message| {
                    message.text.is_empty()
                        || message.text.len() > 1024
                        || message.text.contains(['\0', '\r', '\n'])
                })
        {
            return None;
        }
    }
    Some(data.npcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATA: &str =
        include_str!("../../../../../content/dialogues/quests-2/uneasy-curos-npc-greetings.json");
    #[test]
    fn static_source_bindings_reject_identity_source_and_text_substitution() -> Result<(), String> {
        let bindings = super::super::CUROS_BINDINGS;
        assert_eq!(
            parse(DATA, bindings).ok_or("closed Curos greeting")?.len(),
            1
        );
        for (from, to) in [
            ("oteryn:npc.curos", "oteryn:npc.fake"),
            (
                "6feee8de2475788bca144210fd47cbf857b5dce3a16215ff2647e971bcd3a42e",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            ("Be greeted blank skin.", "Unqualified replacement."),
        ] {
            assert!(parse(&DATA.replace(from, to), bindings).is_none());
        }
        assert!(
            super::super::parse(DATA).is_none(),
            "frozen base23 admission remains closed"
        );
        Ok(())
    }
}
