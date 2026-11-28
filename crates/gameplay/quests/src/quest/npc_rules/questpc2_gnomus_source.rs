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

/// The retained measurements owner predates the separate ordnance programme.
/// Its original source pin and citations remain independent of that supplement.
pub(super) fn legacy_custody(document: &NpcRuleDocument) -> bool {
    let sources = [
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:65-68",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:70-77",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:88-97",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:179-183",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:184-192",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:193-201",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:201-202",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:205-217",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:218-237",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:78-85",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:85-86",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:99-117",
    ];
    document.program_key.is_none()
        && document.quest == QUEST
        && document.source_files.len() == 1
        && document.source_files.iter().all(|file| {
            file.revision == "201189f871d23597b82caf88fd611454a2ddefb8"
                && file.path == PATH
                && file.sha256 == SHA
        })
        && document
            .rules
            .iter()
            .all(|rule| rule.npc == "oteryn:npc.gnomus")
        && document
            .tracks
            .iter()
            .flat_map(|track| &track.sources)
            .chain(document.rules.iter().flat_map(|rule| &rule.sources))
            .all(|source| sources.contains(&source.as_str()))
}

pub(super) fn subterranean_custody(document: &NpcRuleDocument) -> bool {
    const SOURCE: &str =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/klom_stonecutter.lua:";
    document.program_key.as_deref()
        == Some("oteryn:quest-npc-program.dangerous-depths-subterranean-life")
        && document.quest == QUEST
        && document.source_files.len() == 1
        && document.source_files.iter().all(|file| {
            file.path == "data-global/npc/klom_stonecutter.lua"
                && file.revision == "201189f871d23597b82caf88fd611454a2ddefb8"
                && file.sha256 == "80581b78544dd6d62f7fad960365bc9357b51c99989e142765db602f38a9e9b1"
        })
        && document
            .rules
            .iter()
            .all(|rule| rule.npc == "oteryn:npc.klom_stonecutter")
        && document
            .tracks
            .iter()
            .flat_map(|track| &track.sources)
            .chain(document.rules.iter().flat_map(|rule| &rule.sources))
            .all(|source| source.starts_with(SOURCE))
}

#[cfg(test)]
#[path = "questpc2_gnomus_source_tests.rs"]
mod tests;
