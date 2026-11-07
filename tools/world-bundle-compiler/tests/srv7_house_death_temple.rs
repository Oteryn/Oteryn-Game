//! Exact Thais source temple; no synthetic floor or town mask is created.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
use std::path::Path;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_house_death_actual_thais_temple_exports_native_source_bundle() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 125)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let built = compile::compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: vec![],
            families: &project.families,
        },
        &registry,
    )?;
    let data = oteryn_world_bundle::bundle::read(&built.bytes)?;
    let temple = data
        .manifest
        .subareas
        .town(8)
        .and_then(|t| t.temple)
        .ok_or("actual source Thais town8 Temple")?;
    assert_eq!((temple.x, temple.y, temple.floor), (32369, 32241, -7));
    let sector = data
        .sectors
        .iter()
        .find(|s| s.floor == -7 && s.sx == 32369 / 32 && s.sy == 32241 / 32)
        .ok_or("actual Thais temple sector")?;
    let tile = sector
        .tiles
        .iter()
        .find(|t| t.x == 32369 && t.y == 32241)
        .ok_or("actual B3 Thais temple tile")?;
    assert_eq!(tile.house, 0);
    assert!(
        tile.items
            .iter()
            .filter(|i| i.depth == 0)
            .any(|i| data.manifest.palette[i.palette as usize]
                .terrain
                .is_some_and(|t| t.kind == TerrainKind::Ground && t.walkable == Some(true))),
        "actual source temple is walkable"
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_HOUSE_DEATH_BUNDLE") {
        std::fs::write(path, &built.bytes)?;
    }
    eprintln!(
        "SRV7_HOUSE_DEATH_TEMPLE_SOURCE town8=Thais x=32369 y=32241 floor7 real_b3=true real_subarea_masks=true"
    );
    Ok(())
}
