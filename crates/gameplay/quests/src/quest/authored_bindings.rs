//! Explicit native stage bindings for authored recipes. Names and recipe prose never select a target.
//!
//! A binding set belongs to the same immutable catalogue as its tracks and transitions.
//! The ordinary NPC/world/kill owners remain the only execution and durable-write paths.
pub mod actor_admission;
pub mod collection;
pub mod door_use;
pub(crate) mod embedded;
pub mod hits;
mod imported_recipe;
mod occurrence;
pub use occurrence::Occurrence;
pub mod kills;
mod progress_storage;
mod state_operations;
pub mod terrain_use;
pub use kills::KillCounter;

use super::reward_callbacks::RewardCallback;
use super::world_callbacks::Callback;
use super::{QuestStateCatalogue, QuestTrack, QuestTransition};
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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        alternatives: Vec<RewardCallback>,
    },
    CarriedUse {
        callback: Box<RewardCallback>,
    },
    MapUse {
        callback: Box<Callback>,
    },
    StepIn {
        callback: Box<Callback>,
    },
    Kill {
        counter: Box<KillCounter>,
    },
    Hit {
        objective: Box<hits::HitObjective>,
    },
    Collect {
        collection: Box<collection::Collection>,
    },
    TerrainUse {
        usage: Box<terrain_use::TerrainUse>,
    },
    DoorUse {
        usage: Box<door_use::DoorUse>,
    },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<Occurrence>,
    pub admission: Option<actor_admission::ActorAdmission>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuntimeBindings {
    pub(super) source: Bindings,
    pub(super) world: Vec<Callback>,
}

impl Bindings {
    /// Parse a closed, finite native binding document. This alone grants no gameplay authority.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BINDING_BYTES {
            return Err("authored binding byte bound".into());
        }
        let bindings: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        bindings.validate()?;
        Ok(bindings)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || self
                .quests
                .iter()
                .flat_map(|q| &q.stages)
                .filter(|s| matches!(s.trigger, Trigger::Kill { .. }))
                .count()
                > kills::MAX_KILL_BINDINGS
            || self.quests.len() > MAX_BOUND_QUESTS
            || self
                .quests
                .iter()
                .flat_map(|q| &q.stages)
                .map(|s| match &s.trigger {
                    Trigger::NpcDialogue { alternatives, .. } => 1 + alternatives.len(),
                    _ => 1,
                })
                .sum::<usize>()
                + self
                    .quests
                    .iter()
                    .filter(|quest| quest.occurrence.is_some())
                    .count()
                > MAX_BOUND_STAGES
        {
            return Err("authored binding bounds/schema".into());
        }
        let mut quests = BTreeSet::new();
        let mut transitions = BTreeSet::new();
        for quest in &self.quests {
            if !super::valid_quest_key(&quest.quest)
                || !quest.quest.starts_with("oteryn:quest.")
                || !quests.insert(&quest.quest)
                || quest.tracks.is_empty()
                || quest.tracks.len() > 128
                || quest.stages.is_empty()
                || quest.stages.len() > 128
                || !digest(&quest.recipe_sha256)
                || quest.sources.is_empty()
                || quest.sources.len() > 16
                || quest
                    .sources
                    .iter()
                    .any(|s| s.is_empty() || s.len() > 1024 || !s.is_ascii())
            {
                return Err("authored quest identity/source/bounds".into());
            }
            if let Some(occurrence) = &quest.occurrence {
                occurrence.validate(quest)?;
                if !transitions.insert(&occurrence.restart().key) {
                    return Err("authored save restart binding collision".into());
                }
            if let Some(admission) = &quest.admission {
                admission.validate(&quest.quest)?;
            }
            let mut tracks = BTreeSet::new();
            for track in &quest.tracks {
                if !super::valid_quest_key(&track.key)
                    || !tracks.insert(&track.key)
                    || !(track.minimum <= track.initial && track.initial <= track.maximum)
                {
                    return Err("authored track identity/bounds".into());
                }
            }
            let mut stages = BTreeSet::new();
            for stage in &quest.stages {
                if stage.key.is_empty()
                    || stage.key.len() > 128
                    || !stage.key.is_ascii()
                    || !stages.insert(&stage.key)
                {
                    return Err("authored stage identity".into());
                }
                if let Trigger::NpcDialogue { alternatives, .. } = &stage.trigger {
                    if alternatives.len() >= 32 {
                        return Err("TALK callback group bound".into());
                    }
                    for callback in stage.trigger.reward_callbacks() {
                        super::reward_callbacks::validate_callbacks(std::slice::from_ref(
                            callback,
                        ))?;
                        if callback.quest != quest.quest
                            || !transitions.insert(&callback.key)
                            || callback.carried_item.is_some()
                            || !callback.participants.is_empty()
                            || callback
                                .state_ops
                                .iter()
                                .any(|op| !state_operations::valid(quest, op))
                            || callback.storage.is_empty()
                            || callback.storage.iter().any(|s| {
                                s.increment
                                    || s.at_least
                                    || quest.tracks.iter().find(|t| t.key == s.track).is_none_or(
                                        |t| {
                                            s.minimum < t.minimum
                                                || s.maximum > t.maximum
                                                || !(t.minimum..=t.maximum).contains(&s.set)
                                        },
                                    )
                            })
                        {
                            return Err("native TALK foreign or unsupported stage".into());
                        }
                    }
                    continue;
                }
                if let Trigger::Hit { objective } = &stage.trigger {
                    objective.validate(quest)?;
                    if !transitions.insert(&objective.key) {
                        return Err("authored hit binding collision".into());
                if let Trigger::DoorUse { usage } = &stage.trigger {
                    usage.validate(quest)?;
                    if !transitions.insert(&usage.key) {
                        return Err("authored door USE binding collision".into());
                    }
                    continue;
                }
                if let Trigger::TerrainUse { usage } = &stage.trigger {
                    usage.validate(quest)?;
                    if !transitions.insert(&usage.key) {
                        return Err("authored terrain USE binding collision".into());
                    }
                    continue;
                }
                if let Trigger::Collect { collection } = &stage.trigger {
                    collection.validate(quest)?;
                    if !transitions.insert(&collection.key) {
                        return Err("authored collection binding collision".into());
                    }
                    continue;
                }
                if let Trigger::Kill { counter } = &stage.trigger {
                    counter.validate(quest)?;
                    if counter.encounter_source.is_some() && stage.completes {
                        return Err(
                            "encounter deaths cannot complete before the chamber exit".into()
                        );
                    }
                    if !transitions.insert(&counter.key) {
                        return Err("authored kill binding collision".into());
                    }
                    continue;
                }
                let (key, owner, storage) = match &stage.trigger {
                    Trigger::Kill { .. } | Trigger::Collect { .. } | Trigger::Hit { .. } => {
                    Trigger::Kill { .. }
                    | Trigger::Collect { .. }
                    | Trigger::TerrainUse { .. }
                    | Trigger::DoorUse { .. } => {
                        continue;
                    }
                    Trigger::NpcDialogue { callback, .. } | Trigger::CarriedUse { callback } => {
                        super::reward_callbacks::validate_callbacks(std::slice::from_ref(
                            callback.as_ref(),
                        ))?;
                        if !callback.participants.is_empty()
                            || !callback.state_ops.is_empty()
                            || matches!(&stage.trigger, Trigger::NpcDialogue { .. })
                                != callback.carried_item.is_none()
                        {
                            return Err("authored dialogue/carried trigger mismatch".into());
                        }
                        (&callback.key, &callback.quest, &callback.storage)
                    }
                    Trigger::MapUse { callback } | Trigger::StepIn { callback } => {
                        super::world_callbacks::validate_callbacks(std::slice::from_ref(
                            callback.as_ref(),
                        ))?;
                        let expected = if matches!(&stage.trigger, Trigger::MapUse { .. }) {
                            super::world_callbacks::Trigger::Use
                        } else {
                            super::world_callbacks::Trigger::StepIn
                        };
                        if callback.trigger != expected {
                            return Err("authored world trigger mismatch".into());
                        }
                        (&callback.key, &callback.quest, &callback.storage)
                    }
                };
                if owner != &quest.quest
                    || !transitions.insert(key)
                    || (storage.is_empty() && !stage.is_native_encounter())
                    || storage.iter().any(|s| {
                        !tracks.contains(&s.track)
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
                    return Err("authored foreign/unbounded stage".into());
                }
            }
        }
        Ok(())
    }

    /// Compose explicit bindings with existing state. Every binding contributes to its quest hash.
    pub fn attach(&self, base: &QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
        let mut normalized = self.clone();
        for quest in &mut normalized.quests {
            for stage in &mut quest.stages {
                if let Trigger::NpcDialogue {
                    callback,
                    alternatives,
                } = &mut stage.trigger
                {
                    callback.completes = stage.completes;
                    for other in alternatives {
                        other.completes = stage.completes;
                    }
                }
            }
        }
        normalized.attach_normalized(base)
    }
    fn attach_normalized(&self, base: &QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
        self.validate()?;
        if self.quests.is_empty() {
            // An empty addition must retain the existing immutable binding witnesses.
        // Preserve an already-admitted immutable binding set on an empty attachment.
        if self.quests.is_empty() {
            return Ok(base.clone());
        }
        if let Some(evidence) = &base.native_recipe_evidence {
            evidence.verify(self)?;
        }
        let mut tracks: Vec<_> = base.tracks.values().cloned().collect();
        let mut transitions: Vec<_> = base.transitions.values().cloned().collect();
        for quest in &self.quests {
            for t in &quest.tracks {
                tracks.push(QuestTrack {
                    key: t.key.clone(),
                    quest: quest.quest.clone(),
                    initial: t.initial,
                    min: t.minimum,
                    max: t.maximum,
                });
            }
            if let Some(occurrence) = &quest.occurrence {
                transitions.push(occurrence.transition()?);
            }
            for stage in &quest.stages {
                if stage.is_native_encounter() {
                    continue;
                }
                transitions.push(stage.transition(&quest.quest)?);
                let mut transition = stage.transition(&quest.quest)?;
                if let Some(occurrence) = &quest.occurrence {
                    occurrence.qualify(&mut transition);
                }
                transitions.push(transition);
                if let Trigger::NpcDialogue { alternatives, .. } = &stage.trigger {
                    for callback in alternatives {
                        let mut transition = super::reward_callbacks::primary_transition(callback)?;
                        transition.completes = stage.completes;
                        if let Some(occurrence) = &quest.occurrence {
                            occurrence.qualify(&mut transition);
                        }
                        transitions.push(transition);
                    }
                }
            }
        }
        let mut catalogue =
            QuestStateCatalogue::new(base.content_revision.clone(), tracks, transitions)
                .map_err(|e| format!("{e:?}"))?;
        if base.authored_bindings.is_some() {
            return Err("compose all authored bindings in one immutable set".into());
        }
        catalogue.npc_legacy = base.npc_legacy.clone();
        catalogue.native_recipe_evidence = base.native_recipe_evidence.clone();
        catalogue.predecessors = base.predecessors.clone();
        for (key, hash) in &base.hashes {
            if !self.quests.iter().any(|quest| &quest.quest == key) {
                catalogue.hashes.insert(key.clone(), *hash);
            }
        }
        for quest in &self.quests {
            let before = catalogue
                .hashes
                .get_mut(&quest.quest)
                .ok_or("authored quest hash")?;
            let mut digest = Sha256::new()
                .chain_update(*before)
                .chain_update(b"authored-native-bindings-v1");
            if let Some(previous) = base.hashes.get(&quest.quest) {
                digest.update(previous);
            }
            digest.update(serde_json::to_vec(quest).map_err(|e| e.to_string())?);
            *before = digest.finalize().into();
            // Replacing the route requires its new binding witness. Keep the
            // legacy migration owners only for unrelated, retained quests.
            catalogue.npc_legacy.remove(&quest.quest);
            catalogue.predecessors.remove(&quest.quest);
        }
        let mut world = super::world_callbacks::load()?;
        world.extend(self.world_callbacks().cloned());
        if world.len() > MAX_BOUND_STAGES
            || world.iter().map(|c| &c.key).collect::<BTreeSet<_>>().len() != world.len()
        {
            return Err("authored/base world binding collision or bound".into());
        }
        catalogue.authored_bindings = Some(std::sync::Arc::new(RuntimeBindings {
            source: self.clone(),
            world,
        }));
        super::reward_callbacks::load_for(&catalogue)?;
        Ok(catalogue)
    }

    pub fn reward_callbacks(&self) -> impl Iterator<Item = &RewardCallback> {
        self.quests.iter().flat_map(|q| {
            q.occurrence
                .iter()
                .map(Occurrence::restart)
                .chain(q.stages.iter().flat_map(|s| s.trigger.reward_callbacks()))
        })
    }
    pub fn world_callbacks(&self) -> impl Iterator<Item = &Callback> {
        self.quests
            .iter()
            .flat_map(|q| &q.stages)
            .filter_map(|s| match &s.trigger {
                Trigger::MapUse { callback } | Trigger::StepIn { callback } => {
                    Some(callback.as_ref())
                }
                _ => None,
            })
    }
}

impl Stage {
    /// Native entry mutates the physical owner atomically and grants no quest value.
    pub(in crate::quest) fn is_native_encounter(&self) -> bool {
        matches!(&self.trigger, Trigger::MapUse { callback }
            if !self.completes && callback.storage.is_empty() && callback.state_ops.is_empty()
                && callback.effects.encounter.as_ref().is_some_and(|encounter| encounter.timing.is_none()))
    }
    pub(in crate::quest) fn transition(&self, quest: &str) -> Result<QuestTransition, String> {
        let mut transition = match &self.trigger {
            Trigger::NpcDialogue { callback, .. } | Trigger::CarriedUse { callback } => {
                super::reward_callbacks::primary_transition(callback)?
            }
            Trigger::MapUse { callback } | Trigger::StepIn { callback } => {
                super::world_callbacks::primary_transition(callback)?
            }
            Trigger::Kill { counter } => QuestTransition {
                key: counter.key.clone(),
                quest: quest.into(),
                effects: counter.effects(),
                completes: self.completes,
                experience: None,
                world_change: None,
            },
            Trigger::Hit { objective } => objective.transition(quest, self.completes),
            Trigger::Collect { collection } => collection.transition(quest, self.completes)?,
            Trigger::TerrainUse { usage } => usage.transition(quest, self.completes)?,
            Trigger::DoorUse { usage } => usage.transition(quest, self.completes)?,
        };
        transition.completes = self.completes;
        Ok(transition)
    }
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}

#[cfg(test)]
mod asura_palace_tests;
mod actor_admission_tests;

#[cfg(test)]
mod collection_tests;

#[cfg(test)]
mod composition_tests;

#[cfg(test)]
mod encounter_source_tests;
mod occurrence_tests;

impl Trigger {
    fn reward_callbacks(&self) -> impl Iterator<Item = &RewardCallback> {
        let (first, others): (Option<&RewardCallback>, &[RewardCallback]) = match self {
            Self::NpcDialogue {
                callback,
                alternatives,
            } => (Some(callback.as_ref()), alternatives),
            Self::CarriedUse { callback } => (Some(callback.as_ref()), &[]),
            _ => (None, &[]),
        };
        first.into_iter().chain(others.iter())
    }
}

#[cfg(test)]
mod hit_tests;

#[cfg(test)]
mod child_of_destiny_tests;
mod terrain_use_tests;

#[cfg(test)]
mod door_use_tests;

#[cfg(test)]
mod collector_delivery_tests;
