//! The production World callback fence selector handles stateless observations.
#[path = "../src/gameplay_transport/quest_callbacks/admission.rs"]
mod admission;

use admission::{character_fence_required, stateless_observation_required};
use oteryn_gameplay_quests::quest::world_callbacks;

#[test]
fn m02_window_use_acquires_a_character_fence_even_without_personal_state() -> Result<(), String> {
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
