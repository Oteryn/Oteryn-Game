//! Preserve the published Pits programme for characters already in progress.
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, definition_hash};
use std::collections::BTreeMap;

const QUEST: &str = "oteryn:quest.the_pits_of_inferno_quest";
const FOUNTAIN_USE: &str = "oteryn:quest-progress/pits-of-inferno/fountain-used-at";
const SHRINE_VISIT: &str = "oteryn:quest-progress/pits-of-inferno/shrine-crossed";
const FOUNTAIN: &str = "oteryn:quest-callback/cooldmech/fountain";
const SHRINE_ROUTES: [&str; 2] = [
    "oteryn:quest-callback/cooldmech/shrine-north",
    "oteryn:quest-callback/cooldmech/shrine-south",
];

pub(in crate::quest) fn retain_predecessor(catalogue: &mut QuestStateCatalogue) {
    if !catalogue.tracks.contains_key(SHRINE_VISIT) {
        return;
    }
    let tracks = catalogue
        .tracks
        .iter()
        .filter(|(key, track)| {
            track.quest == QUEST && ![FOUNTAIN_USE, SHRINE_VISIT].contains(&key.as_str())
        })
        .map(|(key, track)| (key.clone(), track.clone()))
        .collect();
    let transitions = catalogue
        .transitions
        .iter()
        .filter(|(key, transition)| {
            transition.quest == QUEST
                && key.as_str() != FOUNTAIN
                && !SHRINE_ROUTES.contains(&key.as_str())
        })
        .map(|(key, transition)| (key.clone(), transition.clone()))
        .collect();
    catalogue
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash: definition_hash(QUEST, &tracks, &transitions),
            defaults: BTreeMap::new(),
        });
}
