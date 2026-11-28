use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use oteryn_world_bundle::{bundle, npc::Direction};

#[test]
fn w19a_full_step_2_finds_messenger_in_compiled_world() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Owner-approved positions adjacent to TibiaWiki BR page 28529, revision 416417.
    let expected = [
        (32308_u16, 31837_u16, -8),
        (32356, 32210, -7),
        (32641, 31892, -9),
        (32654, 31665, -8),
        (32916, 32070, -9),
    ];
    let town_regions =
        std::fs::read_to_string(root.join("tools/qualification/node_boot/town-regions.txt"))?;
    let regions: BTreeSet<_> = town_regions
        .trim()
        .split(';')
        .map(str::to_owned)
        .chain(expected.map(|(x, y, _)| format!("{},{}", x / 256, y / 256)))
        .collect();
    let regions = regions.into_iter().collect::<Vec<_>>().join(";");
    let bundle_path = std::env::temp_dir().join(format!("w19a-world-{}.otwb", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_oteryn-world-bundle-compiler"))
        .arg("compile-town-qualification")
        .arg(&root)
        .arg(root.join("content/world/pins/oteryn.identity.json"))
        .arg(&bundle_path)
        .arg(regions)
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&bundle_path)?;
    std::fs::remove_file(bundle_path)?;
    let world = bundle::visit(&bytes, |_, _| Ok(()))?;
    let seller = world
        .npcs
        .npcs
        .iter()
        .find(|entry| entry.key == "oteryn:npc.messenger_of_santa")
        .ok_or("step 2: compiled world has no Messenger of Santa acquisition actor")?;
    let mut actual = Vec::new();
    for placement in &seller.placements {
        assert_eq!(placement.direction, Direction::South);
        actual.push((placement.x, placement.y, placement.floor));
    }
    actual.sort_unstable();
    assert_eq!(actual, expected);
    Ok(())
}
