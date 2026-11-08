//! An inventory collection is a read-only custody predicate, never a hand-in.
use super::{BoundQuest, QuestTransition};
use crate::quest::world_callbacks::Storage;
use crate::quest::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CollectedItem {
    pub item: String,
    pub count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_number: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub key: String,
    pub items: Vec<CollectedItem>,
    pub storage: Vec<Storage>,
    pub sources: Vec<String>,
}

impl Collection {
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with("oteryn:quest-collection/")
            || self.items.is_empty()
            || self.items.len() > 16
            || self.storage.is_empty()
            || self.storage.len() > crate::quest::QUESTSTATE0_RL_02
            || self.sources.is_empty()
            || self.sources.len() > 16
            || self
                .sources
                .iter()
                .any(|source| source.is_empty() || source.len() > 1024)
        {
            return Err("collection identity/bounds/source".into());
        }
        let mut inputs = BTreeSet::new();
        for input in &self.items {
            if !crate::quest::valid_quest_key(&input.item)
                || !input.item.starts_with("oteryn:item.")
                || !(1..=100).contains(&input.count)
                || input.key_number == Some(0)
                || !inputs.insert((&input.item, input.key_number))
                || self.items.iter().any(|other| {
                    other.item == input.item
                        && other.key_number != input.key_number
                        && (input.key_number.is_none() || other.key_number.is_none())
                })
            {
                return Err("collection overlapping/unbounded item predicate".into());
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
                return Err("collection foreign/unbounded track".into());
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
