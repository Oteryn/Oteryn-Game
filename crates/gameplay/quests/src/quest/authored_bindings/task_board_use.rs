//! Bind successful native Task Board programme acceptance, never a client request.
use super::{BoundQuest, QuestTransition};
use crate::quest::world_callbacks::Storage;
use crate::quest::{QuestComparison, QuestEffect};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PREFIX: &str = "oteryn:quest-task-board-use/";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskBoardUse {
    pub key: String,
    pub programme: Programme,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Programme {
    Weekly,
}

impl Programme {
    pub fn accepts(self, action: &crate::quest::task_board::full::TaskBoardAction) -> bool {
        matches!(
            (self, action),
            (
                Self::Weekly,
                crate::quest::task_board::full::TaskBoardAction::StartWeekly { difficulty: 0..=3 }
            )
        )
    }
}

impl TaskBoardUse {
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with(PREFIX)
            || self.storage.is_empty()
            || self.storage.len() > crate::quest::QUESTSTATE0_RL_02
            || self.sources.is_empty()
            || self.sources.len() > 16
            || self.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("Task Board USE identity/source bounds".into());
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
                return Err("Task Board USE foreign or unbounded progress".into());
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
