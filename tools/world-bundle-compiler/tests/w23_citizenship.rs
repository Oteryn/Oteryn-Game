use std::path::Path;

use oteryn_world_bundle_compiler::{b3, bundle};

#[test]
fn w23_authored_temple_portals_keep_the_citizenship_trigger()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/world/placements");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("index.json"))?)?;
    let portal = index["palette"]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .position(|entry| entry["key"] == "oteryn:item.tibia.i1949")
        })
        .ok_or("citizenship portal palette entry")?;
    for (town, (x, y, z), unique) in [
        ("Thais", (32369_u16, 32246_u16, 6), 9057),
        ("Svargrond", (32208, 31134, 7), 9066),
    ] {
        let path = root.join(format!("region-z{z:02}-x{:03}-y{:03}.b3", x / 256, y / 256));
        let bytes = std::fs::read(path)?;
        let mut budget = bundle::BUNDLE_BUDGET;
        let region = b3::decode_region(&bytes, bundle::TILE_LIMITS, &mut budget)?;
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| (tile.x, tile.y) == (x, y))
            .ok_or_else(|| format!("{town} portal tile missing"))?;
        assert!(
            tile.items.iter().any(|item| {
                item.depth == 0
                    && item.palette as usize == portal
                    && item.attrs.unique == Some(unique)
                    && item.attrs.teleport.is_none()
            }),
            "{town} must use its native citizenship trigger"
        );
    }
    Ok(())
}
