#[cfg(unix)]
#[test]
fn node_boot_modes_declare_their_real_scope_floor() -> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::{ChannelId, RuntimeScopeRefV1};
    use crate::gameplay_transport::world_required_capabilities::{
        BoundRequiredCapabilities, offered_for_scope,
    };

    // Execute only the harness's declaration block, before any service or network access.
    let harness = include_str!("../../../../../tools/qualification/node_boot/run.sh");
    let start = harness
        .find("NODE_REQUIRED_CAPABILITIES=")
        .ok_or("declaration")?;
    let end = harness[start..].find("PLATFORM_SOURCE=").ok_or("end")? + start;
    let script = format!(
        "{}\nprintf '%s' \"$NODE_REQUIRED_CAPABILITIES\"",
        &harness[start..end]
    );
    let world = crate::content::native_gameplay_test_world_id()?;
    let channel = ChannelId::decode(&[
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0x03, 0x70, 0x01, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        0x01,
    ])?;
    for (bundle, spells) in [(false, false), (true, false), (false, true)] {
        let output = std::process::Command::new("bash")
            .args(["-eu", "-c", &script])
            .env("NODE_BOOT_WORLD", if bundle { "1" } else { "0" })
            .env("NODE_BOOT_SPELLS", if spells { "1" } else { "0" })
            .output()?;
        assert!(output.status.success());
        let declared: Vec<u32> = serde_json::from_slice(&output.stdout)?;
        assert_eq!(declared.contains(&21), !spells, "keep the logout check");
        let required = BoundRequiredCapabilities::for_channel_boot(
            &declared,
            RuntimeScopeRefV1::channel(world, channel),
            1,
            "nb-world-policy-1",
            crate::gameplay_transport::with_store_offer(offered_for_scope(bundle, None), true),
        )
        .map_err(|error| format!("world={bundle} spells={spells}: {error:?}"))?;
        if bundle {
            for floor in [11, 18, 30] {
                assert!(required.effective().contains(&floor));
            }
        }
    }
    Ok(())
}
