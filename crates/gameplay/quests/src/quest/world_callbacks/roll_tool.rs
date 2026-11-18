//! Crystal 201189f the_dream_courts/actions_strange_bucket.lua:16-59.
use super::{ActorKind, Callback, CallbackEffects, Trigger, roll_reward};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RollToolStage {
    pub source: String,
    pub output: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RollTool {
    pub outcomes: u8,
    pub minimum_success: u8,
    pub rearm_step_ms: u32,
    pub success_effect: u16,
    pub failure_effect: u16,
    pub stages: Vec<RollToolStage>,
}

impl RollTool {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        let Some(tool) = &callback.item_on_target else {
            return false;
        };
        let accepted: BTreeSet<_> = std::iter::once(tool.item.as_str())
            .chain(tool.alternatives.iter().map(String::as_str))
            .collect();
        let stages: BTreeSet<_> = self
            .stages
            .iter()
            .map(|stage| stage.source.as_str())
            .collect();
        let item_key = |key: &str| {
            key.strip_prefix("oteryn:item.tibia.i")
                .and_then(|suffix| suffix.parse::<u32>().ok())
                .is_some_and(|id| id != 0)
        };
        (2..=100).contains(&self.outcomes)
            && (1..self.outcomes).contains(&self.minimum_success)
            && self.rearm_step_ms > 0
            && u64::from(self.outcomes - 1) * u64::from(self.rearm_step_ms) <= 3_600_000
            && (1..=255).contains(&self.success_effect)
            && (1..=255).contains(&self.failure_effect)
            && !self.stages.is_empty()
            && stages.len() == self.stages.len()
            && stages.is_subset(&accepted)
            && accepted.iter().all(|&key| item_key(key))
            && self.stages.iter().all(|stage| {
                item_key(&stage.output)
                    && accepted.contains(stage.output.as_str())
                    && stage.source != stage.output
                    && !stage.text.is_empty()
                    && stage.text.len() <= 256
                    && !stage.text.chars().any(char::is_control)
            })
            && callback.trigger == Trigger::Use
            && callback.actor_kind == ActorKind::Player
            && callback.requires_world_admission
            && callback.selector.native.is_some()
            && callback.lever.is_none()
            && !callback.reversible
            && callback.native_rule.is_none()
            && callback.passages.is_empty()
            && callback.offerings.is_empty()
            && callback.item_reward.is_none()
            && callback.teleport.is_none()
            && !callback.storage.is_empty()
            && callback.storage.iter().all(|track| track.preserve)
            && tool.selected_instance
            && tool.reusable
            && tool.count == 1
            && tool.ingredients.is_empty()
            && tool.target_replacement.is_none()
            && callback.effects.transform.is_some()
            && *callback.effects
                == CallbackEffects {
                    transform: callback.effects.transform,
                    roll_tool: Some(self.clone()),
                    ..Default::default()
                }
    }

    pub fn succeeds(
        &self,
        callback: &str,
        session: &[u8; 16],
        ordinal: u64,
        actor_placement: &[u8; 16],
        movement_revision: u64,
    ) -> bool {
        self.draw(
            callback,
            session,
            ordinal,
            actor_placement,
            movement_revision,
        ) >= u64::from(self.minimum_success)
    }

    pub fn draw(
        &self,
        callback: &str,
        session: &[u8; 16],
        ordinal: u64,
        actor_placement: &[u8; 16],
        movement_revision: u64,
    ) -> u64 {
        roll_reward::draw(
            callback,
            session,
            ordinal,
            actor_placement,
            movement_revision,
        ) % u64::from(self.outcomes.max(1))
    }

    pub fn stage(&self, source: &str) -> Option<&RollToolStage> {
        self.stages.iter().find(|stage| stage.source == source)
    }
}
