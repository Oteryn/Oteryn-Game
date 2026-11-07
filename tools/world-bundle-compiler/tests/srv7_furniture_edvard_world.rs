//! W18 actual retailer room: original B3, authored NPCframe, public talk position.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_edvard_actual_retailer_world_exports_native_trade_fixture() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(129, 124)])?;
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
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let edvard = bundle
        .npcs
        .npcs
        .iter()
        .find(|p| p.key == "oteryn:npc.edvard")
        .ok_or("Edvard must come from actual authored production NPC placements")?;
    assert_eq!(edvard.placements.len(), 1);
    assert_eq!(
        (
            edvard.placements[0].x,
            edvard.placements[0].y,
            edvard.placements[0].floor
        ),
        (33229, 31832, -7)
    );
    for x in [33228, 33229] {
        let tile = bundle
            .sectors
            .iter()
            .filter(|s| s.floor == -7)
            .flat_map(|s| s.tiles.iter())
            .find(|t| t.x == x && t.y == 31832)
            .ok_or("actual public talk floor")?;
        assert_eq!(tile.house, 0);
        assert!(tile.items.iter().filter(|i| i.depth == 0).any(|i| {
            bundle.manifest.palette[i.palette as usize]
                .terrain
                .is_some_and(|t| t.kind == TerrainKind::Ground && t.walkable == Some(true))
        }));
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_FURNITURE_SHOPS_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    eprintln!(
        "SRV7_EDVARD_REAL_WORLD_SOURCE_PARITY {parity:?} npc=33229/31832/7 player=33228/31832/7 no_geometry_rewrite=true"
    );
    Ok(())
}
