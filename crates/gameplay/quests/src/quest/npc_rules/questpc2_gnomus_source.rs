//! Exact retained source custody for the Gnomus ordnance admission program.
use super::NpcRuleDocument;

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-gnomus-ordnance";
pub(super) const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const PATH: &str = "data-global/npc/gnomus.lua";
const SHA: &str = "04ce96e16bbfd9f6fa6b114b377643b1bb71c3df33f05c2b1e1d0d5e40bdc5a1";

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
            .all(|rule| rule.npc == "oteryn:npc.gnomus")
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
        .filter(|reference| reference.starts_with("data-global/npc/gnomus.lua:"))
}

#[cfg(test)]
#[path = "questpc2_gnomus_source_tests.rs"]
mod tests;
