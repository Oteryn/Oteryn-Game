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
    for invalid in 0..5 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.target_definition = Some("oteryn:terrain.tibia.i7535".into()),
            1 => bad.trigger = world_callbacks::Trigger::StepIn,
            2 => bad.item_on_target.as_mut().ok_or("tool")?.selected_instance = false,
            3 => bad.selector.appearance = Some(7535),
            _ => bad.world_requirements.push(world_callbacks::WorldRequirement {
                position: world_callbacks::Position { x: 1, y: 1, floor: -8 },
                appearance: 162,
                ground_stack: None,
                ground: None,
            }),
        }
        assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    }
    Ok(())
}

#[test]
fn koshei_validator_accepts_distinct_tools_and_refuses_overlapping_alternatives() -> TestResult {
    let mut primary = source::callback()?;
    primary.item_on_target.as_mut().ok_or("primary tool")?.alternatives = vec![
        "oteryn:item.tibia.i7533".into(),
    ];
    let mut other = primary.clone();
    other.key.push_str("-different-tool");
    let tool = other.item_on_target.as_mut().ok_or("other tool")?;
    tool.item = "oteryn:item.tibia.i3031".into();
    tool.alternatives.clear();
    world_callbacks::validate_callbacks(&[primary.clone(), other.clone()])?;
    other.item_on_target.as_mut().ok_or("other tool")?.alternatives = vec![
        "oteryn:item.tibia.i7533".into(),
    ];
    assert!(world_callbacks::validate_callbacks(&[primary, other]).is_err());
    Ok(())
}
