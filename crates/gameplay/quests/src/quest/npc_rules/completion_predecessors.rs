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
    // Absent tracks retain their source defaults. Entries without an explicit
    // revision were measured on the complete deployed catalogue at 871c1dec7.
    for (quest, hash) in [
        // Measured at the integrated main revision before Kevin's final promotion
        // became a completion transition. The track definitions are unchanged.
        (
            "oteryn:quest.the_postman_missions_quest",
            [
                0x0a, 0xf6, 0xfe, 0x65, 0xe6, 0x5a, 0xce, 0xfe, 0xb8, 0x5e, 0xda, 0xee, 0x7d, 0x13,
                0x42, 0x51, 0x21, 0x7c, 0xb9, 0xb5, 0xae, 0x12, 0xe9, 0x2c, 0x95, 0xb3, 0x74, 0xd3,
                0x8c, 0xef, 0xf0, 0x5d,
            ],
        ),
        // Measured through the production loader at 7dc142a65.
        (
            "oteryn:quest.the_postman_missions_quest",
            [
                0xc6, 0xe3, 0xda, 0x7c, 0x90, 0xc9, 0xbe, 0xaf, 0xdf, 0x23, 0xc4, 0x06, 0x1a, 0x46,
                0x75, 0x0b, 0xa4, 0x13, 0x93, 0x0a, 0xf6, 0xcd, 0xae, 0xa8, 0x26, 0x19, 0x6c, 0x08,
                0x48, 0x51, 0x6b, 0xe8,
            ],
        ),
        // Measured through the production loader at 35d65457d.
        (
            "oteryn:quest.the_postman_missions_quest",
            [
                0x9f, 0x64, 0x52, 0x8b, 0x3c, 0x6c, 0x06, 0xaf, 0xa5, 0xc5, 0x6d, 0x2c, 0x83, 0xf7,
                0x6c, 0x34, 0xe1, 0x98, 0x45, 0x1f, 0x4f, 0x03, 0x78, 0x69, 0x8c, 0x78, 0x87, 0x02,
                0xdf, 0x5f, 0x83, 0x68,
            ],
        ),
        (
            "oteryn:quest.children_of_the_revolution_quest",
            [
                0x42, 0xf1, 0x9e, 0x44, 0x9f, 0x68, 0x10, 0x42, 0x97, 0x6a, 0x39, 0x81, 0x41, 0xa1,
                0x47, 0xd3, 0xe0, 0xa9, 0xf4, 0x8c, 0x31, 0x2c, 0x7a, 0x75, 0x6f, 0x52, 0x8b, 0x25,
                0xfb, 0x75, 0x20, 0xb4,
            ],
        ),
        (
            "oteryn:quest.grave_danger_quest",
            [
                0x50, 0x54, 0x57, 0x5c, 0xd5, 0x5c, 0x76, 0x48, 0x05, 0x22, 0xbd, 0xc3, 0x1b, 0xab,
                0x71, 0x18, 0x73, 0xfc, 0x7d, 0xd8, 0x59, 0xb3, 0x83, 0x2d, 0x4b, 0xca, 0xcb, 0xee,
                0xed, 0xe0, 0x25, 0x9e,
            ],
        ),
        (
            "oteryn:quest.the_first_dragon_quest",
            [
                0xa4, 0xef, 0x0e, 0x5a, 0x69, 0xd1, 0xe2, 0xb4, 0x0a, 0xe7, 0x8d, 0xc4, 0x2b, 0xd1,
                0xc9, 0x3f, 0x4d, 0xe8, 0xe8, 0xc3, 0xb7, 0x29, 0x61, 0xcc, 0x59, 0x05, 0x58, 0xee,
                0x47, 0xc7, 0xb2, 0x02,
            ],
        ),
    ] {
        if active.definition_hash(quest).is_some() {
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
    fn qpkg3_edron_save_accepts_only_the_deployed_pin() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let quest = "oteryn:quest.grave_danger_quest";
        let mut hash = [
            0x50, 0x54, 0x57, 0x5c, 0xd5, 0x5c, 0x76, 0x48, 0x05, 0x22, 0xbd, 0xc3, 0x1b, 0xab,
            0x71, 0x18, 0x73, 0xfc, 0x7d, 0xd8, 0x59, 0xb3, 0x83, 0x2d, 0x4b, 0xca, 0xcb, 0xee,
            0xed, 0xe0, 0x25, 0x9e,
        ];
        assert_ne!(loaded.catalogue().definition_hash(quest), Some(hash));
        assert!(
            super::super::legacy_defaults(loaded.catalogue(), quest, &hash)
                .ok_or("missing deployed Edron save pin")?
                .is_empty()
        );
        hash[0] ^= 1;
        assert!(super::super::legacy_defaults(loaded.catalogue(), quest, &hash).is_none());
        Ok(())
    }

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
