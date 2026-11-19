use std::path::Path;
use std::process::Command;
use std::time::Instant;

use oteryn_world_bundle::bundle::{self, TerrainKind};

#[test]
fn node_boot_town_bundle_contains_crystal_sellers_and_shop_ground()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let regions =
        std::fs::read_to_string(root.join("tools/qualification/node_boot/town-regions.txt"))?;
    let bundle_path =
        std::env::temp_dir().join(format!("node-boot-sellers-{}.otwb", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_oteryn-world-bundle-compiler"))
        .args(["compile-regions"])
        .arg(&root)
        .arg(root.join("content/world/pins/oteryn.identity.json"))
        .arg(&bundle_path)
        .arg(regions.trim())
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&bundle_path)?;
    std::fs::remove_file(bundle_path)?;
    let started = Instant::now();
    let world = bundle::read(&bytes)?;
    eprintln!(
        "town bundle bytes={} decode_ms={} creature_spawn_points={}",
        bytes.len(),
        started.elapsed().as_millis(),
        world.spawns.point_count()
    );

    for (key, x, y, floor) in [
        ("oteryn:npc.alexander", 33256u16, 31839u16, -3),
        ("oteryn:npc.asima", 33220, 32403, -7),
        ("oteryn:npc.aurelia", 31955, 31916, -7),
    ] {
        assert!(
            world.npcs.npcs.iter().any(|npc| npc.key == key
                && npc
                    .placements
                    .iter()
                    .any(|p| (p.x, p.y, p.floor) == (x, y, floor))),
            "{key} missing at ({x}, {y}, {floor}) from emitted node-boot bundle"
        );
        assert!(
            world
                .sectors
                .iter()
                .filter(|sector| sector.floor == floor)
                .any(|sector| {
                    sector.tiles.iter().any(|tile| {
                        let distance = tile.x.abs_diff(x).max(tile.y.abs_diff(y));
                        distance > 0
                            && distance <= 3
                            && tile.items.iter().any(|item| {
                                world.manifest.palette[item.palette as usize]
                                    .terrain
                                    .is_some_and(|terrain| {
                                        terrain.kind == TerrainKind::Ground
                                            && terrain.walkable == Some(true)
                                    })
                            })
                    })
                }),
            "{key} missing nearby walkable shop ground"
        );
    }
    Ok(())
}
