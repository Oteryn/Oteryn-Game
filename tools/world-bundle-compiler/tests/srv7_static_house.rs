//! W18: genuine fixed House decorations, original House attribution and source masks.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_static_house_real_farm_torch_and_guildhall_fixtures_export_source_bundle() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(127, 124), (126, 121), (127, 123)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: vec![],
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    // Independent source proof checks every tile, flag, zone, House id, attribute and depth.
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let tile = |x, y, floor| {
        bundle
            .sectors
            .iter()
            .filter(|s| s.floor == floor)
            .flat_map(|s| s.tiles.iter())
            .find(|t| t.x == x && t.y == y)
    };
    for (id, x, y, z, house) in [
        (2929, 32655, 31888, 2_i8, 2844),
        (2910, 32291, 31098, 1, 3669),
        (2535, 32650, 31572, 5, 2628),
    ] {
        let source = tile(x, y, -z).ok_or("actual fixed House source tile")?;
        assert_eq!(source.house, house, "source House id cannot be reassigned");
        let key = format!("oteryn:item.tibia.i{id}");
        assert_eq!(
            source
                .items
                .iter()
                .filter(|i| i.depth == 0 && bundle.manifest.palette[i.palette as usize].key == key)
                .count(),
            1,
            "one exact original BASE decoration"
        );
    }
    let entrance = tile(32650, 31890, -2).ok_or("actual source Farm entrance")?;
    assert_eq!(
        entrance.house, 0,
        "source entrance is outside the Farm House"
    );
    assert!(
        entrance
            .items
            .iter()
            .filter(|i| i.depth == 0)
            .any(|i| bundle.manifest.palette[i.palette as usize]
                .terrain
                .is_some_and(|t| t.kind == TerrainKind::Ground && t.walkable == Some(true))),
        "source Farm entrance must have actual admitted ground"
    );
    let door = tile(32651, 31890, -2).ok_or("source Farm door")?;
    assert_eq!(door.house, 2844);
    if let Some(path) = std::env::var_os("OTERYN_SRV7_STATIC_HOUSE_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    eprintln!(
        "SRV7_STATIC_HOUSE_SOURCE_PARITY {parity:?} farm=2844 torch2929=32655/31888/2 entrance=32650/31890/2 guildhalls=3669,2628 original_masks=true"
    );
    Ok(())
}
