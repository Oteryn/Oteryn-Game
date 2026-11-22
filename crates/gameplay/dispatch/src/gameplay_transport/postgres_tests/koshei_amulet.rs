//! Validate the native Ground-target callback rules used by the framed caller.
use super::TestResult;
use oteryn_gameplay_quests::quest::{koshei_amulet as source, world_callbacks};

#[test]
fn koshei_validator_accepts_definition_route_and_refuses_appearance_only() -> TestResult {
    let callback = source::callback()?;
    assert_eq!(callback.effects.target_magic, Some(14));
    world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
    let mut appearance = callback.clone();
    appearance.selector.target_definition = None;
    appearance.selector.appearance = Some(7535);
    appearance.effects.target_magic = None;
    assert!(world_callbacks::validate_callbacks(&[appearance]).is_err());
    for invalid in 0..4 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.target_definition = Some("oteryn:terrain.tibia.i7535".into()),
            1 => bad.trigger = world_callbacks::Trigger::StepIn,
            2 => bad.item_on_target.as_mut().ok_or("tool")?.selected_instance = false,
            _ => bad.selector.appearance = Some(7535),
        }
        assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    }
    Ok(())
}
