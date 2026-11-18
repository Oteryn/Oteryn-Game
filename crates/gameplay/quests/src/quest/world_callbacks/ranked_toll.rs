//! Crystal 201189f dreamers_challenge_quest/movement_tower.lua:2-29.
use super::{ActorKind, Callback, RANKED_TOLL_PREFIX, Trigger};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RankedToll {
    pub tracks: [String; 2],
    pub minimum: i64,
}

impl RankedToll {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        self.minimum > 0
            && self.tracks[0] != self.tracks[1]
            && self.tracks.iter().all(|track| {
                track.starts_with("oteryn:quest-progress/")
                    && track.len() <= 128
                    && super::super::valid_quest_key(track)
            })
            && callback.key.starts_with(RANKED_TOLL_PREFIX)
            && callback.trigger == Trigger::StepIn
            && callback.actor_kind == ActorKind::Player
            && callback.requires_world_admission
            && callback.return_on_failure
            && callback.selector.native.is_some()
            && callback.teleport.is_some()
            && callback.remote_teleport.is_none()
            && !callback.has_personal_state()
            && callback.experience.is_none()
            && callback.monk_reward.is_none()
            && callback.native_rule.is_none()
            && !callback.reversible
            && callback.lever.is_none()
            && callback.offerings.is_empty()
            && callback.passages.is_empty()
            && *callback.effects == super::CallbackEffects {
                ranked_toll: Some(self.clone()),
                teleport_effect: callback.effects.teleport_effect,
                ..Default::default()
            }
            && callback.item_on_target.as_ref().is_some_and(|input| {
                input.on_step
                    && !input.reusable
                    && !input.selected_instance
                    && input.count == 1
                    && input.replacement.is_none()
                    && input.target_replacement.is_none()
                    && !input.target_remove
                    && input.alternatives.is_empty()
                    && input.ingredients.is_empty()
            })
    }
}
