//! Published quest pins accepted by successor definitions. Crystal 201189f8:
//! data-global/npc/albinius.lua:172-192 adds an offering to existing progress;
//! data-global/scripts/quests/ferumbras_ascension/actions_reward.lua:13-38
//! uses an independent claim flag without resetting the preceding quest.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue, authored_bindings};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    predecessors: Vec<Predecessor>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Predecessor {
    quest: String,
    definition_sha256: String,
    #[serde(default)]
    defaults: BTreeMap<String, i64>,
}

pub(super) fn register_embedded(catalogue: &mut QuestStateCatalogue) -> Result<(), String> {
    let documents = authored_bindings::embedded::QUEST_DEFINITION_PREDECESSORS;
    register(catalogue, documents)
}

/// Admit exact historical pins through the catalogue's existing compatibility owner.
/// Validate the entire registry before installing any entries.
pub fn register(
    catalogue: &mut QuestStateCatalogue,
    documents: &[(&str, &str)],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();
    for (path, document) in documents {
        if document.len() > 65_536 {
            return Err(format!("{path}: quest predecessor document size"));
        }
        let Document {
            schema,
            predecessors,
        } = serde_json::from_str(document).map_err(|error| format!("{path}: {error}"))?;
        if schema != 1 || predecessors.is_empty() || predecessors.len() > 64 {
            return Err(format!("{path}: quest predecessor schema or count"));
        }
        for row in predecessors {
            let hash = parse_hash(&row.definition_sha256)
                .ok_or_else(|| format!("{path}: invalid predecessor hash for {}", row.quest))?;
            let valid_defaults = row.defaults.iter().all(|(key, value)| {
                catalogue.track(key).is_some_and(|track| {
                    track.quest == row.quest && (track.min..=track.max).contains(value)
                })
            });
            if !valid_defaults
                || catalogue
                    .definition_hash(&row.quest)
                    .is_none_or(|active| active == hash)
                || !seen.insert((row.quest.clone(), hash))
                || super::npc_rules::legacy_defaults(catalogue, &row.quest, &hash).is_some()
            {
                return Err(format!(
                    "{path}: invalid or duplicate predecessor for {}",
                    row.quest
                ));
            }
            validated.push((
                row.quest,
                NpcDefinitionPredecessor {
                    hash,
                    defaults: row.defaults,
                },
            ));
        }
    }
    for (quest, predecessor) in validated {
        catalogue
            .predecessors
            .entry(quest)
            .or_default()
            .push(predecessor);
    }
    Ok(())
}

fn parse_hash(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    let mut hash = [0; 32];
    for (index, byte) in hash.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(hash)
}
