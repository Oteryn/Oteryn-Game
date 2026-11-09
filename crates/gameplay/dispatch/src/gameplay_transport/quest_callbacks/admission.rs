//! One fence decision for USE, STEP, replay and independent World observations.
use oteryn_gameplay_quests::quest::{carried_step, shared_switches, world_callbacks};
use world_callbacks::Callback;

pub(super) fn stateless_observation_required(callback: &Callback) -> bool {
    !callback.has_personal_state()
        && (callback.effects.source_portal.is_some()
            || world_callbacks::world_change_requirement(&callback.key).is_some()
            || world_callbacks::stateless_ground_requirements(&callback.key).is_some())
}

pub(super) fn character_fence_required(callback: &Callback) -> bool {
    callback.has_personal_state()
        || carried_step::recipe(&callback.key).is_some()
        || shared_switches::recipe(&callback.key).is_some()
        || shared_switches::gate(&callback.key).is_some()
        || stateless_observation_required(callback)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m02_window_use_acquires_a_character_fence_even_without_personal_state() -> Result<(), String>
    {
        let mut window = world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m02/nomads-land/window")
            .ok_or("Nomads window")?;
        assert!(!window.has_personal_state());
        assert!(stateless_observation_required(&window));
        assert!(character_fence_required(&window));
        window.key = "oteryn:quest-callback/fixture-ordinary-map-action".into();
        assert!(!stateless_observation_required(&window));
        assert!(!character_fence_required(&window));
        Ok(())
    }
}
