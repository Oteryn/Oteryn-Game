//! Exact retained source custody for the Albinius temple contribution program.
use super::NpcRuleDocument;

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-albinius-tomes";
pub(super) const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const ICE_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const PATH: &str = "data-global/npc/albinius.lua";
const SHA: &str = "1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18";

pub(super) fn custody(document: &NpcRuleDocument) -> bool {
    // Crystal's Albinius file has identical bytes at both revisions. Keep the
    // original source custody and admit the added Ice Portal witness at 172-192.
    let files = &document.source_files;
    let original = files
        .iter()
        .filter(|file| file.revision == REVISION)
        .count();
    let ice = files
        .iter()
        .filter(|file| file.revision == ICE_REVISION)
        .count();
    document.program_key.as_deref() == Some(PROGRAM)
        && document.quest == QUEST
        && original == 1
        && ice <= 1
        && files.len() == original + ice
        && files
            .iter()
            .all(|file| file.path == PATH && file.sha256 == SHA)
        && document
            .rules
            .iter()
            .all(|rule| rule.npc == "oteryn:npc.albinius")
        && document
            .tracks
            .iter()
            .flat_map(|track| &track.sources)
            .chain(document.rules.iter().flat_map(|rule| &rule.sources))
            .all(|source| {
                reference(QUEST, source).is_some()
                    && (ice == 1 || !source.starts_with(ICE_REFERENCE_PREFIX))
            })
}

const ICE_REFERENCE_PREFIX: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:";

pub(super) fn reference<'a>(quest: &str, source: &'a str) -> Option<&'a str> {
    if quest != QUEST {
        return None;
    }
    source
        .strip_prefix("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
        .filter(|reference| reference.starts_with("data-global/npc/albinius.lua:"))
        .or_else(|| {
            source
                .strip_prefix(ICE_REFERENCE_PREFIX)
                .filter(|reference| *reference == "data-global/npc/albinius.lua:172-192")
        })
}

#[cfg(test)]
#[path = "questpc2_albinius_source_tests.rs"]
mod tests;
