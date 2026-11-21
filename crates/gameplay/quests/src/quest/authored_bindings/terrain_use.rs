//! Read a committed native tool action; this binding never performs a terrain mutation.
use super::{BoundQuest, QuestTransition};
use crate::quest::world_callbacks::Storage;
use crate::quest::{QuestComparison, QuestEffect};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_key: Option<u64>,
    pub result: u32,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}

impl TerrainUse {
    /// An exact map digest plus key, or a stable tile/ordinal key in the fenced World.
    pub fn placement_bytes(&self) -> Option<Vec<u8>> {
        match (&self.placement, self.placement_key) {
            (Some(bytes), None) if bytes.len() == 40 => Some(bytes.clone()),
            (None, Some(key))
                if key >> 48 == 0 && ((key >> 8) & 255) <= 15 && (key & 255) <= 63 =>
            {
                Some(key.to_be_bytes().to_vec())
            }
            _ => None,
        }
    }

    pub fn matches_placement(&self, observed: &[u8]) -> bool {
        if observed.len() != 40 {
            return false;
        }
        match (&self.placement, self.placement_key) {
            (Some(bytes), None) => bytes == observed,
            (None, Some(key)) => observed[32..] == key.to_be_bytes(),
            _ => false,
        }
    }

    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with(PREFIX)
            || self.tools.is_empty()
            || self.tools.len() > 8
            || self.placement_bytes().is_none()
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
                    .is_none_or(|track| !super::progress_storage::valid(storage, track, true))
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
                    effect: super::progress_storage::effect(storage, binding),
                })
                .collect(),
        })
    }
}

impl super::Bindings {
    /// Bounded candidates only; the durable writer independently proves the native receipt.
    pub fn terrain_use_candidates(
        &self,
        catalogue: &crate::quest::QuestStateCatalogue,
        tracks: &std::collections::BTreeMap<String, i64>,
        tool: &ToolDefinition,
        placement: &[u8],
        result: u32,
    ) -> Vec<String> {
        self.quests
            .iter()
            .flat_map(|quest| &quest.stages)
            .filter_map(|stage| {
                let super::Trigger::TerrainUse { usage } = &stage.trigger else {
                    return None;
                };
                let active = usage.tools.contains(tool)
                    && usage.matches_placement(placement)
                    && usage.result == result
                    && usage.storage.iter().all(|guard| {
                        catalogue.track(&guard.track).is_some_and(|track| {
                            let value = tracks.get(&guard.track).copied().unwrap_or(track.initial);
                            (guard.minimum..=guard.maximum).contains(&value)
                        })
                    });
                active.then(|| usage.key.clone())
            })
            .collect()
    }
}
