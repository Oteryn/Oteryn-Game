#[test]
fn node_toml_template_declares_the_floor_each_world_boots_with() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::foundation::{ChannelId, RuntimeScopeRefV1};
    use crate::gameplay_transport::world_required_capabilities::{
        BoundRequiredCapabilities, offered_for_scope,
    };

    // The template's default value and the value its [world_bundle] opt-in prescribes.
    let template = include_str!("../../../../../deploy/synology-game/node.toml.template");
    let default_line = template
        .lines()
        .find(|line| line.starts_with("required_gameplay_capabilities = "))
        .ok_or("default required_gameplay_capabilities")?;
    let fixture: Vec<u32> =
        serde_json::from_str(default_line.trim_start_matches("required_gameplay_capabilities = "))?;
    let opt_in = template
        .lines()
        .filter(|line| line.starts_with('#'))
        .find_map(|line| line.split("required_gameplay_capabilities = ").nth(1))
        .ok_or("bundle opt-in required_gameplay_capabilities")?;
    let opt_in = &opt_in[..=opt_in.find(']').ok_or("opt-in list end")?];
    let bundle_floor: Vec<u32> = serde_json::from_str(opt_in)?;

    let world = crate::content::native_gameplay_test_world_id()?;
    let channel = ChannelId::decode(&[
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0x03, 0x70, 0x01, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        0x02,
    ])?;
    let boot = |declared: &[u32], bundle: bool| {
        BoundRequiredCapabilities::for_channel_boot(
            declared,
            RuntimeScopeRefV1::channel(world, channel),
            1,
            "template-world-policy-1",
            crate::gameplay_transport::with_store_offer(offered_for_scope(bundle, None), true),
        )
    };

    assert_eq!(fixture, Vec::<u32>::new());
    boot(&fixture, false).map_err(|error| format!("fixture room: {error:?}"))?;
    assert_eq!(bundle_floor, vec![11, 18, 30]);
    let required =
        boot(&bundle_floor, true).map_err(|error| format!("bundle world: {error:?}"))?;
    for floor in [11, 18, 30] {
        assert!(required.effective().contains(&floor));
    }
    // The value the template used to give a bundle world fails closed at boot.
    assert!(boot(&fixture, true).is_err());
    Ok(())
}
