//! Crystal 201189f the_dream_courts/actions_sacrophagus_use.lua:25-61.
use super::{ActorKind, Callback, Trigger};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RollReward {
    pub sides: u8,
    pub minimum_success: u8,
    pub failure_creature: String,
    pub failure_effect: u16,
    pub failure_text: String,
    pub busy_text: String,
    pub rearm_ms: u32,
    pub source_action: u16,
}

impl RollReward {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        (2..=100).contains(&self.sides)
            && (2..=self.sides).contains(&self.minimum_success)
            && self.failure_creature.starts_with("oteryn:creature.")
            && super::super::valid_quest_key(&self.failure_creature)
            && (1..=255).contains(&self.failure_effect)
            && !self.failure_text.is_empty()
            && self.failure_text.len() <= 512
            && !(self.failure_text.chars().any(char::is_control))
            && !self.busy_text.is_empty()
            && self.busy_text.len() <= 512
            && !(self.busy_text.chars().any(char::is_control))
            && (1..=600_000).contains(&self.rearm_ms)
            && self.source_action != 0
            && callback.trigger == Trigger::Use
            && callback.actor_kind == ActorKind::Player
            && callback.requires_world_admission
            && callback.selector.native.is_some()
            && callback.lever.is_none()
            && !callback.reversible
            && callback.native_rule.is_none()
            && callback.passages.is_empty()
            && callback.offerings.is_empty()
            && callback.item_on_target.is_none()
            && callback.teleport.is_none()
            && !callback.storage.is_empty()
            && callback.storage.iter().all(|track| track.preserve)
            && *callback.effects
                == super::CallbackEffects {
                    item_reward: callback.effects.item_reward.clone(),
                    roll_reward: Some(self.clone()),
                    ..Default::default()
                }
            && callback.effects.item_reward.is_some()
    }

    /// Replay draws from the same fenced occurrence and exact actor placement.
    pub fn succeeds(
        &self,
        callback: &str,
        session: &[u8; 16],
        ordinal: u64,
        actor_placement: &[u8; 16],
        movement_revision: u64,
    ) -> bool {
        let draw = 1 + draw(
            callback,
            session,
            ordinal,
            actor_placement,
            movement_revision,
        ) % u64::from(self.sides.max(1));
        draw >= u64::from(self.minimum_success)
    }
}

pub(super) fn draw(
    callback: &str,
    session: &[u8; 16],
    ordinal: u64,
    actor_placement: &[u8; 16],
    movement_revision: u64,
) -> u64 {
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-world-reward-roll:v1")
        .chain_update(session)
        .chain_update(ordinal.to_be_bytes())
        .chain_update(actor_placement)
        .chain_update(movement_revision.to_be_bytes())
        .chain_update(callback.as_bytes())
        .finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}
