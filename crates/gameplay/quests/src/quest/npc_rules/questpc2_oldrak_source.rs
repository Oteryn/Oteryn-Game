//! Exact retained source custody for the Oldrak Demon Oak program.
use super::NpcRuleDocument;

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-oldrak-demon-oak";
pub(super) const QUEST: &str = "oteryn:quest.demon_oak";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const PATH: &str = "data-global/npc/oldrak.lua";
const SHA: &str = "dc124275de082d9f39bc2f802b31f823646548153fe0e248417dc54f7b0eba40";

pub(super) fn custody(document: &NpcRuleDocument) -> bool {
    document.program_key.as_deref() == Some(PROGRAM)
        && document.quest == QUEST
        && document.source_files.len() == 1
        && document
            .source_files
            .iter()
            .all(|file| file.revision == REVISION && file.path == PATH && file.sha256 == SHA)
        && document
            .rules
            .iter()
            .all(|rule| rule.npc == "oteryn:npc.oldrak")
        && document
            .tracks
            .iter()
            .flat_map(|track| &track.sources)
            .chain(document.rules.iter().flat_map(|rule| &rule.sources))
            .all(|source| reference(QUEST, source).is_some())
}

pub(super) fn reference<'a>(quest: &str, source: &'a str) -> Option<&'a str> {
    if quest != QUEST {
        return None;
    }
    source
        .strip_prefix("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
        .filter(|reference| reference.starts_with("data-global/npc/oldrak.lua:"))
}

#[cfg(test)]
#[path = "questpc2_oldrak_source_tests.rs"]
mod tests;
