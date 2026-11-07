//! Closed admission for the hash-pinned Draconia greeting callback.
use super::{Document, Greeting, PIN, Source};
const HANDLER: &str = "data/npclib/npc_system/npc_handler.lua";
const HANDLER_SHA: &str = "8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a";
fn source(source: &Source, path: &str, sha: &str, line: u32) -> bool {
    source.repository == "zimbadev/crystalserver"
        && source.revision == PIN
        && source.path == path
        && source.sha256 == sha
        && source.line == line
}
pub(super) fn parse(text: &str) -> Option<Vec<Greeting>> {
    let data: Document = serde_json::from_str(text).ok()?;
    if data.schema != 1
        || data.source_revision != PIN
        || data.npcs.len() != 1
        || !source(&data.placeholder_source, HANDLER, HANDLER_SHA, 383)
    {
        return None;
    }
    let entry = data.npcs.first()?;
    if entry.npc != "oteryn:npc.an_old_dragonlord"
        || !entry.variants.is_empty()
        || !source(
            &entry.greet.source,
            "data-global/npc/an_old_dragon_lord.lua",
            "85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052",
            67,
        )
        || !source(&entry.farewell.source, HANDLER, HANDLER_SHA, 82)
        || !source(&entry.walkaway.source, HANDLER, HANDLER_SHA, 90)
        || [&entry.greet, &entry.farewell, &entry.walkaway]
            .into_iter()
            .any(|m| {
                m.text.is_empty() || m.text.len() > 1024 || m.text.contains(['\0', '\r', '\n'])
            })
    {
        return None;
    }
    Some(data.npcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATA: &str =
        include_str!("../../../../../content/dialogues/quests-2/draconia-npc-greetings.json");
    #[test]
    fn draconia_greeting_rejects_source_or_identity_substitution() -> Result<(), String> {
        assert_eq!(parse(DATA).ok_or("closed greeting")?.len(), 1);
        for (from, to) in [
            ("oteryn:npc.an_old_dragonlord", "oteryn:npc.fake"),
            (
                "85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ] {
            assert!(parse(&DATA.replace(from, to)).is_none());
        }
        assert!(super::super::parse(DATA).is_none());
        Ok(())
    }
}
