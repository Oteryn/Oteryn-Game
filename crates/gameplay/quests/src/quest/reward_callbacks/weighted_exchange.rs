//! Authored alternatives for a paid NPC turn-in.
use super::{ItemReward, RewardCallback, valid_item_reward};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WeightedExchange {
    /// An active save-scoped world change permits one exchange per character and occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_change: Option<String>,
    pub outcomes: Vec<WeightedOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WeightedOutcome {
    pub weight: u16,
    pub give: ItemReward,
    pub reply: Vec<String>,
}

impl WeightedExchange {
    pub fn total_weight(&self) -> i64 {
        self.outcomes
            .iter()
            .map(|outcome| i64::from(outcome.weight))
            .sum()
    }

    /// Zero has no outcome. Each positive ticket occupies exactly one bucket.
    pub fn outcome(&self, mut ticket: i64) -> Option<&WeightedOutcome> {
        if ticket <= 0 {
            return None;
        }
        self.outcomes.iter().find(|outcome| {
            ticket -= i64::from(outcome.weight);
            ticket <= 0
        })
    }

    pub(super) fn valid(&self, callback: &RewardCallback) -> bool {
        callback.give.is_none()
            && callback.world_claim.is_none()
            && callback.carried_item.is_none()
            && !callback.take.is_empty()
            && !callback.repeatable
            && !callback.per_occurrence_mint
            && callback.fee.is_none()
            && callback.counter.is_none()
            && callback.state_ops.is_empty()
            && callback.participants.is_empty()
            && callback.experience.is_none()
            && callback.cosmetic_targets().next().is_none()
            && callback.achievement.is_none()
            && callback.achievement_progress.is_empty()
            && !callback.storage.is_empty()
            && match &self.world_change {
                Some(change) => {
                    !change.is_empty()
                        && change.len() <= 64
                        && change
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                        && !callback.completes
                        && callback.storage.len() <= 8
                        && callback.storage.iter().all(|state| state.preserve)
                }
                None => callback.storage.iter().any(|state| {
                    !state.preserve && !(state.minimum..=state.maximum).contains(&state.set)
                }),
            }
            && (2..=8).contains(&self.outcomes.len())
            && self.outcomes.iter().all(|outcome| {
                outcome.weight > 0
                    && valid_item_reward(&outcome.give)
                    && !outcome.give.premium
                    && !outcome.reply.is_empty()
                    && outcome.reply.len() <= 8
                    && outcome.reply.iter().all(|line| {
                        !line.is_empty() && line.len() <= 512 && !line.chars().any(char::is_control)
                    })
            })
    }
}
