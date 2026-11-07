//! Actual compiler-produced source B3 must retain donor ground projectile facts.
use super::*;
#[cfg(test)]
mod actual_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/world-bundle-compiler/tests/support/halloween_hare_fixture.rs"
    ));
}
#[test]
fn srv7_hare_actual_b3_terrain_ground_has_source_projectile_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = if let Some(path) = std::env::var_os("OTERYN_SRV7_HARE_BUNDLE") {
        std::fs::read(path)?
    } else {
        actual_fixture::compile(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))?
    };
    let bundle = oteryn_world_bundle::bundle::read(&bytes)?;
    let pins = BundlePins {
        digest: bundle.digest,
        project_format_version: bundle.manifest.identity.project_format_version.clone(),
        world_schema_version: bundle.manifest.identity.world_schema_version.clone(),
        content_revision: bundle.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = Arc::new(crate::map::load(&bytes, &pins)?);
    let mut builder = SpellTilesBuilder::new(base);
    let masks = vec![0; bundle.manifest.palette.len()];
    for sector in &bundle.sectors {
        builder.push(
            &bundle.manifest,
            sector,
            &BTreeSet::new(),
            &|_| None,
            &masks,
        );
    }
    let tiles = builder.finish(&DoorBook::default())?;
    for x in [32624, 32625] {
        let tile = tiles
            .lookup(LogicalCell { x, y: 32018, z: 7 })
            .map_err(|e| format!("actual source tile: {e:?}"))?;
        assert!(
            tile.ground_present(),
            "actual source B3 ground remains present"
        );
        assert!(
            !tile.flags().block_solid,
            "actual source ground remains walkable"
        );
        assert!(
            !tile.flags().block_projectile,
            "pinned4515 grass cannot block every Hare area spell"
        );
    }
    eprintln!(
        "SRV7_HARE_ACTUAL_B3_GROUND_EXECUTED source_and_observer=true source_projectile=false no_portable_item_profile=true"
    );
    Ok(())
}
