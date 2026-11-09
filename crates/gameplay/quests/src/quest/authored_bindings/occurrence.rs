//! Explicit save-scoped authored missions; the durable writer supplies current authority.
use super::{BoundQuest, QuestTransition, RewardCallback};
use crate::quest::QuestComparison;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Occurrence {
    ServerSave {
        world_change: String,
        restart: Box<RewardCallback>,
    },
}

impl Occurrence {
    pub(super) fn restart(&self) -> &RewardCallback {
        match self {
            Self::ServerSave { restart, .. } => restart,
        }
    }
    pub(super) fn gate(&self) -> &str {
        match self {
            Self::ServerSave { world_change, .. } => world_change,
        }
    }
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        let callback = self.restart();
        if self.gate().is_empty()
            || self.gate().len() > 128
            || !self
                .gate()
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_@".contains(&b))
            || callback.storage.is_empty()
            || callback.storage.len() > crate::quest::QUESTSTATE0_RL_02
        {
            return Err("authored save occurrence gate/reset bounds".into());
        }
        let mut tracks = BTreeSet::new();
        for storage in &callback.storage {
            let declared = quest
                .tracks
                .iter()
                .find(|track| track.key == storage.track)
                .ok_or("save reset requires an owned track")?;
            if !tracks.insert(&storage.track)
                || storage.minimum != declared.minimum
                || storage.maximum != declared.maximum
                || storage.set != declared.initial
                || storage.preserve
                || storage.increment
                || storage.at_least
            {
                return Err("save reset must restore declared mission defaults".into());
            }
        }
        if !callback.is_save_restart_only() {
            return Err("authored save restart must be a pure owned NPC reset".into());
        }
        crate::quest::reward_callbacks::validate_callbacks(std::slice::from_ref(callback))
    }
    pub(super) fn qualify(&self, transition: &mut QuestTransition) {
        transition.world_change = Some(self.gate().into());
    }
    pub(super) fn transition(&self) -> Result<QuestTransition, String> {
        let mut transition = crate::quest::reward_callbacks::primary_transition(self.restart())?;
        for effect in &mut transition.effects {
            effect.from = QuestComparison::Any;
        }
        self.qualify(&mut transition);
        Ok(transition)
    }
}
