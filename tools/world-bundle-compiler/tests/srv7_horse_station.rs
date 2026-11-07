//! Exact source B3 for both rental stations, two escape areas and31159 Horse arrivals.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
use std::collections::BTreeMap;
use std::path::Path;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_horse_station_exports_actual_two_stations_and_catch_grass() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 125), (126, 126), (128, 125)])?;
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
    let profiles: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    let physics = profiles["records"]
        .as_array()
        .ok_or("item profiles")?
        .iter()
        .filter_map(|row| {
            Some((
                row["production_binding"]["target"]["key"]
                    .as_str()?
                    .to_owned(),
                row["attributes"]["blocks_movement"].as_bool()?,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut source = BTreeMap::new();
    for sector in &data.sectors {
        for tile in &sector.tiles {
            if sector.floor != -7 {
                continue;
            }
            let mut open = true;
            let mut ground = false;
            let mut marker = false;
            for item in &tile.items {
                let p = &data.manifest.palette[item.palette as usize];
                if let Some(t) = p.terrain {
                    ground |= t.kind == oteryn_world_bundle_compiler::bundle::TerrainKind::Ground
                        && t.walkable == Some(true);
                    open &= t.kind != oteryn_world_bundle_compiler::bundle::TerrainKind::Wall
                        && t.walkable != Some(false)
                } else if !physics.contains_key(&p.key) {
                    open = false
                }
                // Native physical facts also constrain Terrain-backed palette
                // entries. Terrain walkability alone does not clear a solid
                // source fence whose exact Item profile is admitted.
                if let Some(blocks) = physics.get(&p.key) {
                    open &= !*blocks;
                }
                marker |= p.key == "oteryn:item.tibia.i31159";
            }
            source.insert(
                (i32::from(tile.x), i32::from(tile.y)),
                (ground && open, marker),
            );
        }
    }
    for (key, x, y) in [
        ("oteryn:npc.appaloosa", 32852_u16, 32124_u16),
        ("oteryn:npc.palomino", 32450_u16, 32226_u16),
    ] {
        assert!(
            source
                .get(&(i32::from(x), i32::from(y)))
                .is_some_and(|p| p.0),
            "actual station ground{key}"
        );
        assert!(
            project
                .families
                .npc_placements
                .iter()
                .any(|p| p.npc == key && p.cell == (x, y, 7)),
            "production source station placement{key}"
        );
    }
    let markers = source.iter().filter(|(_, p)| p.1).collect::<Vec<_>>();
    assert_eq!(markers.len(), 11, "all real source31159 tiles retained");
    assert_eq!(
        markers.iter().filter(|(_, p)| p.0).count(),
        10,
        "one real marker remains source-solid2174; no geometry invented"
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_HORSE_BUNDLE") {
        std::fs::write(path, &built.bytes)?;
    }
    eprintln!(
        "SRV7_HORSE_STATION_SOURCE_B3 stations=2 catch_markers=11 reachable_markers=10 original_regions=3 geometry_rewritten=false"
    );
    Ok(())
}
