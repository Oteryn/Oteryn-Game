//! Crystal startup selectors must be present on the shipped native carriers.
use oteryn_world_bundle::{bundle, sector::Attrs};
use oteryn_world_bundle_compiler::{b3, compile, source};
use std::{error::Error, path::Path};

fn source_attrs(x: u16, y: u16, z: u8, appearance: u16) -> Result<Attrs, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let index = source::json(
        &source::read(&root.join("content/world/placements/index.json"))?,
        "placement index",
    )?;
    let path = root.join(format!(
        "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
        x / 256,
        y / 256
    ));
    let mut budget = bundle::BUNDLE_BUDGET;
    let region = b3::decode_region(&source::read(&path)?, bundle::TILE_LIMITS, &mut budget)?;
    let mut items = region
        .sectors
        .into_iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|tile| tile.x == x && tile.y == y && tile.house == 0)
        .flat_map(|tile| tile.items)
        .filter(|item| {
            item.depth == 0
                && index["palette"][item.palette as usize]["source_item_id"] == appearance
        });
    let item = items.next().ok_or("missing native carrier")?;
    assert!(
        items.next().is_none(),
        "ambiguous native carrier at ({x}, {y}, {z})"
    );
    Ok(item.attrs)
}

#[test]
fn dead_archer_corpse_has_source_uid() -> Result<(), Box<dyn Error>> {
    assert_eq!(source_attrs(32514, 32303, 10, 4240)?.unique, Some(6169));
    Ok(())
}

#[test]
fn fibula_native_caches_have_source_uids() -> Result<(), Box<dyn Error>> {
    for (x, y, item, uid) in [
        (32219, 32401, 387, 5000),
        (32239, 32471, 4024, 6008),
        (32239, 32478, 4024, 6009),
        (32233, 32491, 4240, 6010),
        (32245, 32492, 4025, 6011),
        (32256, 32500, 4240, 6012),
    ] {
        assert_eq!(source_attrs(x, y, 10, item)?.unique, Some(uid), "UID {uid}");
    }
    Ok(())
}

#[test]
fn darashia_lair_has_both_level_40_gates() -> Result<(), Box<dyn Error>> {
    for x in [33265, 33266] {
        assert_eq!(source_attrs(x, 32278, 7, 5293)?.action, Some(1040));
    }
    Ok(())
}

#[test]
fn demona_approach_has_level_60_gate() -> Result<(), Box<dyn Error>> {
    assert_eq!(source_attrs(32483, 31722, 15, 5111)?.action, Some(1060));
    Ok(())
}

#[test]
fn fibula_forcefields_have_both_native_return_destinations() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(125, 126)])?;
    let parity = compile::parity(&project.regions, &project.families)?;
    assert_eq!(parity.teleport_attributes, parity.teleports_matched);
    for (x, y) in [(32208, 32433), (32234, 32502)] {
        assert_eq!(
            source_attrs(x, y, 10, 1949)?.teleport,
            Some((32167, 32438, 9))
        );
    }
    Ok(())
}

#[test]
fn dead_archer_approach_has_source_shovel_hole_and_lower_descents() -> Result<(), Box<dyn Error>> {
    assert_eq!(source_attrs(32493, 32259, 7, 593)?.unique, None);
    for (x, y, z, appearance) in [
        (32498, 32260, 8, 385),
        (32511, 32257, 9, 432),
        // The lower passage briefly crosses the region boundary to y < 32256.
        (32512, 32255, 10, 429),
        (32513, 32254, 10, 353),
        (32517, 32255, 10, 352),
    ] {
        assert_eq!(source_attrs(x, y, z, appearance)?.teleport, None);
    }
    Ok(())
}
