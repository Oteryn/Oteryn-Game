//! Exact retained source custody and predecessor for Gerimor.
use super::{NpcRuleCatalog, NpcRuleDocument, admission};
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, definition_hash};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.questpc2-gerimor-assignments";
pub(super) const QUEST: &str = "oteryn:quest.cults_of_tibia_quest";
const REVISION: &str = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d";
const FINALE: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const ESSENCE_ENTRY: &str = "oteryn:quest-callback/engenc2/cults-essence-entry";
const ESSENCE_CLOCK: &str = "oteryn:quest-progress/cults-of-tibia/essence-entered-at";
const PATH: &str = "data-global/npc/gerimor.lua";
const SHA: &str = "bf0fba86d07bb8a70d6242c2cc748fbb137cc7b583b47f0886530b6b5c584b2f";

pub(super) fn custody(document: &NpcRuleDocument) -> bool {
    document.program_key.as_deref() == Some(PROGRAM)
        && document.quest == QUEST
        && document.source_files.len() == 2
        && document
            .source_files
            .iter()
            .zip([REVISION, FINALE])
            .all(|(file, revision)| {
                file.revision == revision && file.path == PATH && file.sha256 == SHA
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
            .all(|source| reference(QUEST, source).is_some())
}

pub(super) fn reference<'a>(quest: &str, source: &'a str) -> Option<&'a str> {
    if quest != QUEST {
        return None;
    }
    source
        .strip_prefix("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
        .or_else(|| source.strip_prefix("crystal:201189f871d23597b82caf88fd611454a2ddefb8:"))
        .filter(|reference| reference.starts_with("data-global/npc/gerimor.lua:"))
}

/// The old programme and its original Minotaur bound form the published assignment pin.
pub(crate) fn retain_predecessor(catalogue: &mut QuestStateCatalogue) -> Result<(), String> {
    let source = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    let Some(program) = source.program(QUEST) else {
        return Ok(());
    };
    let current_key = admission::binding_key(program);
    let Some(marker) = catalogue.transition(&current_key) else {
        return Ok(());
    };
    let mut previous = program.clone();
    previous
        .document
        .source_files
        .retain(|file| file.revision == REVISION);
    let old_reference =
        |source: &String| source.starts_with("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:");
    previous
        .document
        .rules
        .retain(|rule| rule.sources.iter().all(old_reference));
    previous
        .document
        .tracks
        .retain(|track| track.sources.iter().all(old_reference));
    previous.digest =
        Sha256::digest(serde_json::to_vec(&previous.document).map_err(|error| error.to_string())?)
            .into();
    let mut tracks: BTreeMap<_, _> = catalogue
        .tracks
        .iter()
        .filter(|(key, track)| track.quest == QUEST && key.as_str() != ESSENCE_CLOCK)
        .map(|(key, track)| (key.clone(), track.clone()))
        .collect();
    let minotaurs = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/minotaurs/mission";
    tracks
        .get_mut(minotaurs)
        .ok_or("missing Gerimor Minotaur track")?
        .max = 5;
    let mut transitions: BTreeMap<_, _> = catalogue
        .transitions
        .iter()
        .filter(|(key, transition)| {
            transition.quest == QUEST
                && key.as_str() != current_key
                && key.as_str() != ESSENCE_ENTRY
        })
        .map(|(key, transition)| (key.clone(), transition.clone()))
        .collect();
    let mut marker = marker.clone();
    marker.key = admission::binding_key(&previous);
    transitions.insert(marker.key.clone(), marker);
    catalogue
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash: definition_hash(QUEST, &tracks, &transitions),
            defaults: BTreeMap::new(),
        });
    Ok(())
}

#[cfg(test)]
#[path = "questpc2_gerimor_source_tests.rs"]
mod tests;
