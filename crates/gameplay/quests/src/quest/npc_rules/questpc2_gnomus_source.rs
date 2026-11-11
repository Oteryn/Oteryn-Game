//! Exact source custody and evaluation order for the two retained Gnomus task programs.
use super::{NpcRuleCatalog, NpcRuleDocument, NpcRuleInput, NpcRuleProgram, NpcRuleTrigger};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-gnomus-ordnance";
pub(super) const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const PATH: &str = "data-global/npc/gnomus.lua";
const SHA: &str = "04ce96e16bbfd9f6fa6b114b377643b1bb71c3df33f05c2b1e1d0d5e40bdc5a1";

pub(super) fn custody(document: &NpcRuleDocument) -> bool {
    if document.program_key.is_none() {
        return legacy_custody(document);
    }
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

const MEASUREMENTS_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const MEASUREMENTS_DOCUMENT: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json"
);

fn measurements_program() -> Option<&'static NpcRuleProgram> {
    static SOURCE: OnceLock<Option<NpcRuleProgram>> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let document: NpcRuleDocument = serde_json::from_str(MEASUREMENTS_DOCUMENT).ok()?;
            let canonical = serde_json::to_vec(&document).ok()?;
            Some(NpcRuleProgram {
                document,
                digest: Sha256::digest(canonical).into(),
            })
        })
        .as_ref()
}

fn measurements_custody(document: &NpcRuleDocument) -> bool {
    document.schema == 1
        && document.program_key.is_none()
        && document.quest == QUEST
        && document.source_files.len() == 1
        && document.source_files.iter().all(|file| {
            file.revision == MEASUREMENTS_REVISION && file.path == PATH && file.sha256 == SHA
        })
        // This separate retained capture owns only its exact guarded report/cooldown.
        // It must not become a guardless program that hides the ordnance safety reset.
        && measurements_program().is_some_and(|source| document == &source.document)
}

/// Preserve the guarded measurements owner before the ordnance lane's unsupported-task
/// reset, independently of document order. The ordinary evaluator still checks every
/// topic and predicate. If measurements has no plan, the reset remains authoritative.
pub(super) fn evaluation_order<'a>(
    source: &'a NpcRuleCatalog,
    input: &NpcRuleInput<'_>,
) -> impl Iterator<Item = &'a NpcRuleProgram> {
    let measurements = if input.npc == "oteryn:npc.gnomus"
        && input.trigger == NpcRuleTrigger::Say
        && source.programs.iter().any(|program| {
            program.document.program_key.as_deref() == Some(PROGRAM) && custody(&program.document)
        }) {
        measurements_program().and_then(|expected| {
            source.programs.iter().position(|program| {
                program.document.program_key.is_none()
                    && program.document.quest == QUEST
                    // with_native_bounds retains the admitted source digest while
                    // widening only the shared native track ranges.
                    && program.digest == expected.digest
            })
        })
    } else {
        None
    };
    measurements
        .into_iter()
        .map(move |index| &source.programs[index])
        .chain(
            source
                .programs
                .iter()
                .enumerate()
                .filter_map(move |(index, program)| {
                    (Some(index) != measurements).then_some(program)
                }),
        )
}

/// The retained measurements owner predates the separate ordnance programme.
/// Its original source pin and citations remain independent of that supplement.
pub(super) fn legacy_custody(document: &NpcRuleDocument) -> bool {
    let sources = [
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:65-68",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:88-97",
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
        // Retain the exact accepted guarded programme, including effects and topics.
        && measurements_custody(document)
}

#[cfg(test)]
#[path = "questpc2_gnomus_source_tests.rs"]
mod tests;
