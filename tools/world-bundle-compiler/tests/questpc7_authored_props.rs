//! Source-qualified static props must survive the real map compiler and equivalence proof.
use oteryn_world_bundle_compiler::{
    b3,
    bundle::{self, BuildClass, Identity},
    compile::{self, Input, KeyResolver, Resolution},
    source,
};
use std::{collections::BTreeSet, error::Error};
#[test]
fn feaster_source_props_compile_and_retain_native_item_identity() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/authored-questpc7.json"),
    )?)?;
    let props = doc["props"].as_array().ok_or("props")?;
    assert_eq!(props.len(), 6);
    let mut selected = BTreeSet::new();
    for prop in props {
        let p = &prop["position"];
        selected.insert((
            u16::try_from(p["x"].as_u64().ok_or("x")?)? / 256,
            u16::try_from(p["y"].as_u64().ok_or("y")?)? / 256,
        ));
    }
    let project = source::project_regions(&root, &selected.into_iter().collect::<Vec<_>>())?;
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
    for prop in props {
        let key = prop["item"].as_str().ok_or("item")?;
        assert!(
            matches!(registry.resolve(key), Resolution::Resolved(_, _)),
            "unresolved prop {key}"
        );
        let at = u32::try_from(palette.iter().position(|k| k == key).ok_or("palette")?)?;
        let p = &prop["position"];
        let mut count = 0;
        for bytes in &project.regions {
            let mut budget = bundle::BUNDLE_BUDGET;
            let region = b3::decode_region(bytes, bundle::TILE_LIMITS, &mut budget)?;
            if -i64::from(region.z) != p["floor"].as_i64().ok_or("floor")? {
                continue;
            }
            for tile in region.sectors.iter().flat_map(|(_, _, tiles)| tiles) {
                if u64::from(tile.x) == p["x"].as_u64().ok_or("x")?
                    && u64::from(tile.y) == p["y"].as_u64().ok_or("y")?
                {
                    count += tile
                        .items
                        .iter()
                        .filter(|i| i.depth == 0 && i.palette == at)
                        .count();
                }
            }
        }
        assert_eq!(count, 1, "exact native prop {key}");
    }
    let compiled = compile::compile(&input, &registry)?;
    compile::equivalence(&input, &registry, &compiled.bytes)?;
    eprintln!(
        "QUESTPC7_AUTHORED_PROPS six source static props compiled and exact native map equivalence PASS"
    );
    Ok(())
}
