//! One fence decision for USE, STEP, replay and independent World observations.
use oteryn_gameplay_quests::quest::{carried_step, shared_switches, world_callbacks};
use world_callbacks::Callback;

pub(super) fn stateless_observation_required(callback: &Callback) -> bool {
    !callback.has_personal_state()
        && (world_callbacks::world_change_requirement(&callback.key).is_some()
            || world_callbacks::stateless_ground_requirements(&callback.key).is_some())
}

pub(super) fn character_fence_required(callback: &Callback) -> bool {
    callback.requires_durable_commit()
        || callback.effects.reward_door.is_some()
        || carried_step::recipe(&callback.key).is_some()
        || shared_switches::recipe(&callback.key).is_some()
        || shared_switches::gate(&callback.key).is_some()
        || stateless_observation_required(callback)
}
