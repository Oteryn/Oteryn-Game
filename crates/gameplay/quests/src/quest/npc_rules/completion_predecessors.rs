//! Completion metadata preserves exact old NPC pins and existing absent-track semantics.
use super::{NpcRuleCatalog, admission};
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) fn register(
    active: &mut QuestStateCatalogue,
    source: &NpcRuleCatalog,
) -> Result<(), String> {
    for quest in [
        "oteryn:quest.a_father_s_burden_quest",
        "oteryn:quest.marlin_trophy_quest",
    ] {
        let mut transitions = active.transitions.clone();
        let mut changed = false;
        for program in source
            .programs()
            .iter()
            .filter(|p| p.document.quest == quest)
        {
            if !program.document.rules.iter().any(|rule| rule.completes) {
                continue;
            }
            let mut previous = program.clone();
            for rule in &mut previous.document.rules {
                rule.completes = false;
            }
            previous.digest =
                Sha256::digest(serde_json::to_vec(&previous.document).map_err(|e| e.to_string())?)
                    .into();
            let mut marker = transitions
                .remove(&admission::binding_key(program))
                .ok_or("missing current completion marker")?;
            marker.key = admission::binding_key(&previous);
            if transitions.insert(marker.key.clone(), marker).is_some() {
                return Err("duplicate completion predecessor marker".into());
            }
            changed = true;
        }
        if changed {
            let previous = QuestStateCatalogue::new(
                active.content_revision.clone(),
                active.tracks.values().cloned().collect(),
                transitions.into_values().collect(),
            )
            .map_err(|e| format!("{e:?}"))?;
            let hash = previous
                .definition_hash(quest)
                .ok_or("missing completion predecessor quest")?;
            // These NPC definitions changed only metadata: leaving missing rows
            // missing retains both Native defaults and Lua's source sentinel.
            active
                .predecessors
                .entry(quest.into())
                .or_default()
                .push(NpcDefinitionPredecessor {
                    hash,
                    defaults: BTreeMap::new(),
                });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn m04_terminal_metadata_accepts_only_exact_previous_pins() -> Result<(), String> {
        let baseline: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("fixtures/m04_pre_completion_hashes.json"))
                .map_err(|e| e.to_string())?;
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        for (quest, text) in baseline {
            let mut hash = [0u8; 32];
            for (i, byte) in hash.iter_mut().enumerate() {
                *byte =
                    u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
            }
            assert_ne!(loaded.catalogue().definition_hash(&quest), Some(hash));
            assert!(
                super::super::legacy_defaults(loaded.catalogue(), &quest, &hash)
                    .ok_or("missing exact predecessor")?
                    .is_empty()
            );
            hash[0] ^= 1;
            assert!(super::super::legacy_defaults(loaded.catalogue(), &quest, &hash).is_none());
            let source = super::super::NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
            assert_eq!(
                source
                    .program(&quest)
                    .ok_or("programme")?
                    .document
                    .rules
                    .iter()
                    .filter(|r| r.completes)
                    .count(),
                1
            );
        }
        Ok(())
    }
}
