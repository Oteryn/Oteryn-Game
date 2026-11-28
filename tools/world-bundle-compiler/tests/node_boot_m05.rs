use std::path::Path;
use std::process::Command;

use oteryn_world_bundle::bundle;

#[test]
fn m05_batch_adds_one_public_closed_basic_door() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let regions =
        std::fs::read_to_string(root.join("tools/qualification/node_boot/town-regions.txt"))?;
    let path = std::env::temp_dir().join(format!("node-boot-m05-{}.otwb", std::process::id()));
    for (batch, expected) in [("131:1", 0), ("132:1", 1)] {
        let result = Command::new(env!("CARGO_BIN_EXE_oteryn-world-bundle-compiler"))
            .env("OTERYN_E2E_ROW_BATCH", batch)
            .arg("compile-town-qualification")
            .arg(&root)
            .arg(root.join("content/world/pins/oteryn.identity.json"))
            .arg(&path)
            .arg(format!("{};125,124;131,122", regions.trim()))
            .output()?;
        assert!(
            result.status.success(),
            "batch {batch}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = std::fs::read(&path)?;
        std::fs::remove_file(&path)?;
        let mut actual = 0;
        bundle::visit(&bytes, |manifest, sector| {
            for tile in sector.tiles.into_iter().filter(|tile| tile.house == 0) {
                actual += tile
                    .items
                    .iter()
                    .filter(|item| {
                        item.depth == 0
                            && manifest.palette[item.palette as usize].key
                                == "oteryn:item.tibia.i1638"
                    })
                    .count();
            }
            Ok(())
        })?;
        assert_eq!(actual, expected, "public basic doors for batch {batch}");
    }
    Ok(())
}
