//! W18: actual global easels and current owned-House entrance share one source bundle.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
#[test]
fn srv7_artist_easel_real_source_bundle_preserves_pickup_and_house_entrance()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(131, 124), (126, 128)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let compiled = compile::compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    let bundle = oteryn_world_bundle_compiler::bundle::read(&compiled.bytes)?;
    let tile = |x, y, z| {
        bundle
            .sectors
            .iter()
            .filter(|s| s.floor == z)
            .flat_map(|s| s.tiles.iter())
            .find(|t| t.x == x && t.y == y)
    };
    for x in [33572, 33573] {
        let t = tile(x, 31867, -12).ok_or("real easel tile")?;
        assert_eq!(t.house, 0);
        assert!(
            t.items
                .iter()
                .any(|i| bundle.manifest.palette[i.palette as usize].key
                    == "oteryn:item.tibia.i14687")
        );
    }
    assert_eq!(tile(32309, 32804, -7).ok_or("real House door")?.house, 3437);
    let interior = tile(32310, 32803, -7).ok_or("real open House interior")?;
    assert_eq!(interior.house, 3437);
    assert!(
        interior
            .items
            .iter()
            .any(|item| bundle.manifest.palette[item.palette as usize].key
                == "oteryn:item.tibia.i408")
    );
    assert!(!interior.items.iter().any(
        |item| bundle.manifest.palette[item.palette as usize].key == "oteryn:item.tibia.i1357"
    ));
    if let Some(path) = std::env::var_os("OTERYN_SRV7_EASEL_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    Ok(())
}
