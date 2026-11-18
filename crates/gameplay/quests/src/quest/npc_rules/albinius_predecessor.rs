//! Keep an exact pre-Ice quest definition readable by the fenced NPC writer.
use super::{NpcRuleCatalog, admission, questpc2_albinius_source};
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, definition_hash};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// Canonical NPC programme digest for the four-offering Albinius document on the
// pre-Ice base (871c1dec7). This pins every retained branch and its source order.
const PREVIOUS_BINDING: &str =
    "oteryn:quest-npc-pin/88afeeac40c70dd69e212857997adfec587df9d23aae007cfd062271dba1e8a2";
const ICE: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_ice";

fn ice_source(source: &str) -> bool {
    questpc2_albinius_source::reference(questpc2_albinius_source::QUEST, source)
        .and_then(|reference| reference.rsplit_once(':'))
        .map(|(_, span)| span)
        .is_some_and(|span| {
            let (first, last) = span.split_once('-').unwrap_or((span, span));
            matches!((first.parse::<u32>(), last.parse::<u32>()),
                (Ok(first), Ok(last)) if 172 <= first && first <= last && last <= 192)
        })
}

pub(in crate::quest) fn register(active: &mut QuestStateCatalogue) -> Result<(), String> {
    let quest = questpc2_albinius_source::QUEST;
    let source = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    let Some(program) = source.program(quest) else {
        return Ok(());
    };
    let mut previous = program.clone();
    previous
        .document
        .rules
        .retain(|rule| !rule.sources.iter().all(|s| ice_source(s)));
    if previous.document.rules.len() == program.document.rules.len() {
        return Ok(());
    }
    previous.document.tracks.retain(|track| track.key != ICE);
    for rule in &mut previous.document.rules {
        // The old source generator treated any message mentioning Ice Portal
        // as unowned. Restore this exclusion at its original position.
        if rule
            .exclude_keywords
            .get(2)
            .is_none_or(|keyword| keyword != "ice portal")
            && rule.exclude_keywords.len() >= 2
        {
            rule.exclude_keywords.insert(2, "ice portal".into());
        }
    }
    previous.digest =
        Sha256::digest(serde_json::to_vec(&previous.document).map_err(|error| error.to_string())?)
            .into();
    if admission::binding_key(&previous) != PREVIOUS_BINDING {
        return Ok(());
    }
    // AccessIce was already declared by the native mission catalogue.
    if !active.track(ICE).is_some_and(|track| {
        track.quest == quest && (track.initial, track.min, track.max) == (0, 0, 1)
    }) {
        return Err("Albinius Ice predecessor changed native AccessIce semantics".into());
    }
    let Some(mut marker) = active.transition(&admission::binding_key(program)).cloned() else {
        return Ok(());
    };
    let mut transitions: BTreeMap<_, _> = active
        .transitions
        .iter()
        .filter(|(_, transition)| transition.quest == quest && transition.key != marker.key)
        .map(|(key, transition)| (key.clone(), transition.clone()))
        .collect();
    marker.key = PREVIOUS_BINDING.into();
    let first = previous
        .document
        .tracks
        .first()
        .ok_or("Albinius predecessor has no tracks")?;
    marker
        .effects
        .first_mut()
        .ok_or("Albinius marker has no effect")?
        .track
        .clone_from(&first.key);
    if transitions.insert(marker.key.clone(), marker).is_some() {
        return Err("duplicate Albinius predecessor marker".into());
    }
    let hash = definition_hash(quest, &active.tracks, &transitions);
    active
        .predecessors
        .entry(quest.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: BTreeMap::new(),
        });
    Ok(())
}
