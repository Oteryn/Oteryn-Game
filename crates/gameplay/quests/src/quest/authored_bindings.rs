//! M01a: explicit NPC TALK bindings, admitted against the canonical chosen recipe.
pub(crate) mod embedded;
mod imported_recipe;
mod state_operations;
use super::reward_callbacks::RewardCallback;
use super::{QuestStateCatalogue, QuestTrack};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
pub const MAX_BINDING_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BOUND_QUESTS: usize = 128;
pub const MAX_BOUND_STAGES: usize = 1024;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub key: String,
    pub initial: i64,
    pub minimum: i64,
    pub maximum: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    NpcDialogue {
        callback: Box<RewardCallback>,
        /// Other exact NPCs participating in this same chosen TALK stage.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        alternatives: Vec<RewardCallback>,
    },
}

impl Trigger {
    fn callbacks(&self) -> impl Iterator<Item = &RewardCallback> {
        let Self::NpcDialogue {
            callback,
            alternatives,
        } = self;
        std::iter::once(callback.as_ref()).chain(alternatives.iter())
    }
    fn callbacks_mut(&mut self) -> impl Iterator<Item = &mut RewardCallback> {
        let Self::NpcDialogue {
            callback,
            alternatives,
        } = self;
        std::iter::once(callback.as_mut()).chain(alternatives.iter_mut())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub key: String,
    pub completes: bool,
    pub trigger: Trigger,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoundQuest {
    pub quest: String,
    pub recipe_sha256: String,
    pub tracks: Vec<Track>,
    pub stages: Vec<Stage>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub schema: u16,
    pub quests: Vec<BoundQuest>,
}

impl Bindings {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BINDING_BYTES {
            return Err("native TALK byte bound".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || self.quests.len() > MAX_BOUND_QUESTS
            || self.reward_callbacks().count() > MAX_BOUND_STAGES
        {
            return Err("native TALK schema/bounds".into());
        }
        let mut owners = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for quest in &self.quests {
            if !super::valid_quest_key(&quest.quest)
                || !owners.insert(&quest.quest)
                || quest.sources.is_empty()
                || quest.tracks.is_empty()
                || quest.stages.is_empty()
                || quest.recipe_sha256.len() != 64
                || !quest
                    .recipe_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err("native TALK owner/evidence".into());
            }
            let mut tracks = BTreeSet::new();
            for track in &quest.tracks {
                if !track.key.starts_with("oteryn:quest-progress/")
                    || !super::valid_quest_key(&track.key)
                    || !tracks.insert(&track.key)
                    || !(track.minimum <= track.initial && track.initial <= track.maximum)
                {
                    return Err("native TALK track".into());
                }
            }
            let mut stages = BTreeSet::new();
            for stage in &quest.stages {
                if stage.key.is_empty()
                    || stage.key.len() > 128
                    || !stage.key.is_ascii()
                    || !stages.insert(&stage.key)
                    || stage.trigger.callbacks().count() > 32
                {
                    return Err("native TALK stage identity/bound".into());
                }
                for callback in stage.trigger.callbacks() {
                    super::reward_callbacks::validate_callbacks(std::slice::from_ref(callback))?;
                    if callback.quest != quest.quest
                        || !keys.insert(&callback.key)
                        || callback.carried_item.is_some()
                        || !callback.participants.is_empty()
                        || callback
                            .state_ops
                            .iter()
                            .any(|operation| !state_operations::valid(quest, operation))
                        || callback.storage.is_empty()
                        || callback.storage.iter().any(|s| {
                            s.increment
                                || s.at_least
                                || quest
                                    .tracks
                                    .iter()
                                    .find(|t| t.key == s.track)
                                    .is_none_or(|t| {
                                        s.minimum < t.minimum
                                            || s.maximum > t.maximum
                                            || !(t.minimum..=t.maximum).contains(&s.set)
                                    })
                        })
                    {
                        return Err("native TALK foreign or unsupported stage".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub fn attach(&self, base: &QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
        self.validate()?;
        if let Some(evidence) = &base.native_recipe_evidence {
            evidence.verify(self)?;
        }
        if base.authored_bindings.is_some() {
            return Err("compose TALK bindings in one snapshot".into());
        }
        let mut normalized = self.clone();
        let mut tracks: Vec<_> = base.tracks.values().cloned().collect();
        let mut transitions: Vec<_> = base.transitions.values().cloned().collect();
        for quest in &mut normalized.quests {
            tracks.extend(quest.tracks.iter().map(|t| QuestTrack {
                key: t.key.clone(),
                quest: quest.quest.clone(),
                initial: t.initial,
                min: t.minimum,
                max: t.maximum,
            }));
            for stage in &mut quest.stages {
                for callback in stage.trigger.callbacks_mut() {
                    callback.completes = stage.completes;
                    transitions.push(super::reward_callbacks::primary_transition(callback)?);
                }
            }
        }
        let mut catalogue =
            QuestStateCatalogue::new(base.content_revision.clone(), tracks, transitions)
                .map_err(|e| format!("{e:?}"))?;
        catalogue.npc_legacy = base.npc_legacy.clone();
        catalogue.predecessors = base.predecessors.clone();
        catalogue.native_recipe_evidence = base.native_recipe_evidence.clone();
        for (key, hash) in &base.hashes {
            if !self.quests.iter().any(|q| &q.quest == key) {
                catalogue.hashes.insert(key.clone(), *hash);
            }
        }
        for quest in &normalized.quests {
            let before = catalogue
                .hashes
                .get_mut(&quest.quest)
                .ok_or("native TALK hash")?;
            let mut hash = Sha256::new()
                .chain_update(*before)
                .chain_update(b"native-talk-bindings-v1");
            if let Some(previous) = base.hashes.get(&quest.quest) {
                hash.update(previous);
            }
            hash.update(serde_json::to_vec(quest).map_err(|e| e.to_string())?);
            *before = hash.finalize().into();
            catalogue.npc_legacy.remove(&quest.quest);
            catalogue.predecessors.remove(&quest.quest);
        }
        catalogue.authored_bindings = Some(std::sync::Arc::new(normalized));
        super::reward_callbacks::load_for(&catalogue)?;
        Ok(catalogue)
    }
    pub fn reward_callbacks(&self) -> impl Iterator<Item = &RewardCallback> {
        self.quests
            .iter()
            .flat_map(|q| &q.stages)
            .flat_map(|s| s.trigger.callbacks())
    }
}
