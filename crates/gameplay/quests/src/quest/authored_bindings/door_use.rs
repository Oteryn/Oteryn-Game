//! Bind native key-door openings; parsing never grants world-action authority.
use super::{BoundQuest, QuestTransition};
use crate::quest::world_callbacks::Storage;
use crate::quest::{QuestComparison, QuestEffect};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const PREFIX: &str = "oteryn:quest-door-use/";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DoorUse {
    pub key: String,
    /// Native bundle codec, including its signed floor byte; not a terrain-use key.
    pub native_placement_key: u64,
    pub key_number: u16,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}

impl DoorUse {
    pub fn matches_opening(&self, placement: &[u8], key_number: u16) -> bool {
        placement.len() == 40
            && placement[32..] == self.native_placement_key.to_be_bytes()
            && key_number == self.key_number
    }

    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with(PREFIX)
            || self.native_placement_key >> 48 != 0
            || self.native_placement_key & 255 > 63
            || self.key_number == 0
            || self.storage.is_empty()
            || self.storage.len() > crate::quest::QUESTSTATE0_RL_02
            || self.sources.is_empty()
            || self.sources.len() > 16
            || self.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("door USE identity/placement/key/source bounds".into());
        }
        let mut tracks = BTreeSet::new();
        for storage in &self.storage {
            if !tracks.insert(&storage.track)
                || quest
                    .tracks
                    .iter()
                    .find(|t| t.key == storage.track)
                    .is_none_or(|track| !super::progress_storage::valid(storage, track, true))
            {
                return Err("door USE foreign or unbounded progress".into());
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
            Sha256::digest(serde_json::to_vec(self).map_err(|e| e.to_string())?).into();
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
    /// Candidates only. Root must independently admit a captured native opening.
    pub fn door_use_candidates(
        &self,
        catalogue: &crate::quest::QuestStateCatalogue,
        tracks: &BTreeMap<String, i64>,
        placement: &[u8],
        key_number: u16,
    ) -> Vec<String> {
        self.quests
            .iter()
            .flat_map(|q| &q.stages)
            .filter_map(|stage| {
                let super::Trigger::DoorUse { usage } = &stage.trigger else {
                    return None;
                };
                (usage.matches_opening(placement, key_number)
                    && usage.storage.iter().all(|guard| {
                        catalogue.track(&guard.track).is_some_and(|track| {
                            let value = tracks.get(&guard.track).copied().unwrap_or(track.initial);
                            (guard.minimum..=guard.maximum).contains(&value)
                        })
                    }))
                .then(|| usage.key.clone())
            })
            .collect()
    }
}
