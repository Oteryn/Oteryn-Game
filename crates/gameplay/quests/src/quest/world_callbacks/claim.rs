//! The existing Quest tracks and receipts are the durable claim ledger.
use super::{ActorKind, Callback, Trigger};
use crate::quest::{
    reward_callbacks::{ItemReward, valid_item_reward},
    valid_quest_key,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Restore both live pools to their current maxima and save the same projection.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restore_vitals: bool,
    /// Uniform choice between the eligible complete bundles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rewards: Vec<ClaimReward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_track: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<ClaimMilestone>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimReward {
    pub give: ItemReward,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_owned_mount: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimMilestone {
    pub track: String,
    pub count: i64,
    pub achievement: String,
}

impl Callback {
    pub fn needs_item_reward_admission(&self) -> bool {
        self.item_reward().is_some()
            || self
                .effects
                .claim
                .as_ref()
                .is_some_and(|claim| !claim.rewards.is_empty())
    }
}

impl Claim {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        callback.requires_world_admission
            && callback.actor_kind == ActorKind::Player
            && matches!(callback.trigger, Trigger::Use | Trigger::StepIn)
            && callback.has_state_writes()
            && !callback.is_read_only()
            && !callback.reversible
            && !callback.opening_only
            && callback.native_rule.is_none()
            && callback.item_reward().is_none()
            && (self.restore_vitals || self.milestone.is_some() || !self.rewards.is_empty())
            && self.rewards.len() <= super::MAX_EFFECTS
            && (self.rewards.is_empty() || callback.trigger == Trigger::Use)
            && (!self.restore_vitals
                || (callback.trigger == Trigger::Use && callback.teleport.is_none()))
            && self.rewards.is_empty() == self.selection_track.is_none()
            && self.selection_track.as_ref().is_none_or(|key| {
                callback.storage.iter().any(|track| {
                    track.track == *key
                        && track.minimum == 0
                        && track.maximum == self.rewards.len() as i64
                        && track.set == 0
                        && !track.increment
                        && !track.at_least
                        && !track.preserve
                        && !track.read_only
                        && track.set_if.is_none()
                        && track.add.is_none()
                })
            })
            && self.rewards.iter().all(|reward| {
                valid_item_reward(&reward.give)
                    && reward.exclude_owned_mount.as_ref().is_none_or(|mount| {
                        valid_quest_key(mount) && mount.starts_with("oteryn:mount.")
                    })
                    && reward
                        .message
                        .as_ref()
                        .is_none_or(|text| !text.is_empty() && text.len() <= 512)
            })
            && self.milestone.as_ref().is_none_or(|milestone| {
                milestone.count > 0
                    && valid_quest_key(&milestone.achievement)
                    && milestone.achievement.starts_with("oteryn:achievement.")
                    && callback.storage.iter().any(|track| {
                        track.track == milestone.track && track.increment && track.set == 1
                    })
            })
    }
}
