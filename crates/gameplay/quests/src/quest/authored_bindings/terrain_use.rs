//! Read a committed native tool action; this binding never performs a terrain mutation.
use super::{BoundQuest, QuestTransition};
use crate::quest::world_callbacks::Storage;
use crate::quest::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PREFIX: &str = "oteryn:quest-terrain-use/";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    pub key: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerrainUse {
    pub key: String,
    pub tools: Vec<ToolDefinition>,
    pub placement: Vec<u8>,
    pub result: u32,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}

impl TerrainUse {
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with(PREFIX)
            || self.tools.is_empty()
            || self.tools.len() > 8
            || self.placement.len() != 40
            || self.result == 0
            || self.storage.is_empty()
            || self.storage.len() > crate::quest::QUESTSTATE0_RL_02
            || self.sources.is_empty()
            || self.sources.len() > 16
            || self
                .sources
                .iter()
                .any(|source| source.is_empty() || source.len() > 1024)
        {
            return Err("terrain USE identity/bounds/source".into());
        }
        let mut tools = BTreeSet::new();
        for tool in &self.tools {
            if !crate::quest::valid_quest_key(&tool.key)
                || !tool.key.starts_with("oteryn:item.")
                || tool.revision.is_empty()
                || tool.revision.len() > 512
                || !tools.insert((&tool.key, &tool.revision))
            {
                return Err("terrain USE ambiguous/unbounded tool definition".into());
            }
        }
        let mut tracks = BTreeSet::new();
        for storage in &self.storage {
            if !tracks.insert(&storage.track)
                || storage.minimum > storage.maximum
                || quest
                    .tracks
                    .iter()
                    .find(|track| track.key == storage.track)
                    .is_none_or(|track| {
                        storage.minimum < track.minimum
                            || storage.maximum > track.maximum
                            || !(track.minimum..=track.maximum).contains(&storage.set)
                    })
            {
                return Err("terrain USE foreign/unbounded track".into());
            }
        }
        Ok(())
    }

    pub(in crate::quest) fn transition(
        &self,
        quest: &str,
        completes: bool,
    ) -> Result<QuestTransition, String> {
        let binding: [u8; 32] =
            Sha256::digest(serde_json::to_vec(self).map_err(|error| error.to_string())?).into();
        Ok(QuestTransition {
            key: self.key.clone(),
            quest: quest.into(),
            completes,
            experience: None,
            world_change: None,
            effects: self
                .storage
                .iter()
                .map(|storage| QuestEffect {
                    track: storage.track.clone(),
                    from: QuestComparison::Between(storage.minimum, storage.maximum),
                    effect: if storage.preserve {
                        QuestEffectKind::Add(0)
                    } else {
                        QuestEffectKind::SetWithBinding {
                            value: storage.set,
                            binding,
                        }
                    },
                })
                .collect(),
        })
    }
}
