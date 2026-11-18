//! Exact retained source custody for the Gerimor assignment program.
use super::{NpcRuleCatalog, NpcRuleDocument, admission, bounds};
use crate::quest::{
    NpcDefinitionPredecessor, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTransition,
};

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-gerimor-assignments";
pub(super) const REPORT_PROGRAM: &str = "oteryn:quest-npc-program.gerimor-reports";
pub(super) const QUEST: &str = "oteryn:quest.cults_of_tibia_quest";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const PATH: &str = "data-global/npc/gerimor.lua";
const SHA: &str = "bf0fba86d07bb8a70d6242c2cc748fbb137cc7b583b47f0886530b6b5c584b2f";

pub(super) fn reports(document: &NpcRuleDocument) -> bool {
    document.program_key.as_deref() == Some(REPORT_PROGRAM)
}

/// Crystal 201189f8 data-global/npc/gerimor.lua:356-447 adds reports/finale to the
/// published assignment programme. Register the exact prior definition before
/// inserting the new programme pin and widening any existing report's range.
pub(super) fn extend(
    active: &mut QuestStateCatalogue,
    source: &NpcRuleCatalog,
) -> Result<(), String> {
    let Some(program) = source.programs().iter().find(|p| reports(&p.document)) else {
        return Ok(());
    };
    let hash = active
        .definition_hash(QUEST)
        .ok_or("missing Gerimor assignment definition")?;
    let writes = bounds::literal_write_bounds(program);
    for track in &program.document.tracks {
        let target = active
            .tracks
            .get_mut(&track.key)
            .ok_or("unknown Gerimor report track")?;
        if target.quest != QUEST || target.initial != track.initial {
            return Err(format!(
                "Gerimor report track differs from predecessor: {}",
                track.key
            ));
        }
        if let Some(&(min, max)) = writes.get(&track.key) {
            target.min = target.min.min(min);
            target.max = target.max.max(max);
        }
    }
    let first = program
        .document
        .tracks
        .first()
        .ok_or("empty Gerimor report programme")?;
    let key = admission::binding_key(program);
    if active
        .transitions
        .insert(
            key.clone(),
            QuestTransition {
                key,
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: first.key.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
                world_change: None,
            },
        )
        .is_some()
    {
        return Err("duplicate Gerimor report programme pin".into());
    }
    let next = crate::quest::definition_hash(QUEST, &active.tracks, &active.transitions);
    active.hashes.insert(QUEST.into(), next);
    active
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: Default::default(),
        });
    Ok(())
}

pub(super) fn reports_custody(document: &NpcRuleDocument) -> bool {
    // The assignment Lua file is byte-identical in 9f5a72c6 and 201189f8.
    reports(document)
        && !document.rules.is_empty()
        && !document.tracks.is_empty()
        && document.quest == QUEST
        && document.schema == 1
        && document.source_files.len() == 1
        && document.source_files.iter().all(|file| {
            file.revision == "201189f871d23597b82caf88fd611454a2ddefb8"
                && file.path == PATH
                && file.sha256 == SHA
        })
        && document
            .rules
            .iter()
            .all(|rule| rule.npc == "oteryn:npc.gerimor")
        && document
            .tracks
            .iter()
            .flat_map(|track| &track.sources)
            .chain(document.rules.iter().flat_map(|rule| &rule.sources))
            .all(|source| {
                source
                    .strip_prefix(super::SOURCE_PIN)
                    .is_some_and(|reference| reference.starts_with("data-global/npc/gerimor.lua:"))
            })
}

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
            .all(|rule| rule.npc == "oteryn:npc.gerimor")
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
        .filter(|reference| reference.starts_with("data-global/npc/gerimor.lua:"))
}

#[cfg(test)]
#[path = "questpc2_gerimor_source_tests.rs"]
mod tests;
