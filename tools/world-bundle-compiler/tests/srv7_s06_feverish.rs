//! S06 natural source: the unmodified active Venore Feverish Citizen raid area.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
use std::{collections::BTreeSet, path::Path};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_s06_feverish_raid_actual_native_ground_exports_source_bundle() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/encounters/raids/catalogue.json"),
    )?)?;
    let raid = catalogue["records"]
        .as_array()
        .ok_or("raid records")?
        .iter()
        .find(|r| r["key"] == "raid:venore.feverishcitizens")
        .ok_or("natural active raid")?;
    assert_eq!(raid["active"], true);
    let entry = &raid["variants"][0]["waves"][0]["entries"][0];
    assert_eq!(entry["creature"], "oteryn:creature.feverish_citizen");
    assert_eq!(entry["amount"], serde_json::json!({"min":120,"max":120}));
    assert_eq!(
        entry["area"]["from"],
        serde_json::json!({"x":32861,"y":32052,"floor":6})
    );
    assert_eq!(
        entry["area"]["to"],
        serde_json::json!({"x":33012,"y":32126,"floor":6})
    );
    assert_eq!(entry["source"]["line"], 5);
    let project = source::project_regions(&root, &[(128, 125)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let built = compile::compile(
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
    let bundle = oteryn_world_bundle::bundle::read(&built.bytes)?;
    let cells: BTreeSet<_> = bundle
        .sectors
        .iter()
        .filter(|s| s.floor == -6)
        .flat_map(|s| &s.tiles)
        .filter(|t| {
            (32861..=33012).contains(&t.x)
                && (32052..=32126).contains(&t.y)
                && t.flags == 0
                && t.house == 0
                && t.zones.is_empty()
                && t.items.len() == 1
                && t.items[0].depth == 0
                && bundle.manifest.palette[t.items[0].palette as usize]
                    .terrain
                    .is_some_and(|p| p.kind == TerrainKind::Ground && p.walkable == Some(true))
        })
        .map(|t| (t.x, t.y))
        .collect();
    let &(x, y) = cells
        .iter()
        .find(|&&(x, y)| x < 33012 && cells.contains(&(x + 1, y)))
        .ok_or("two adjacent real walkable non-house non-PZ raid cells")?;
    let positions = serde_json::json!({"player":[x,y,6],"creature":[x+1,y,6],"raid":"raid:venore.feverishcitizens","source_region":"content/world/placements/region-z06-x128-y125.b3","source_xml":"data-global/raids/venore/feverish_citizens.xml:5"});
    if let Some(path) = std::env::var_os("OTERYN_SRV7_S06_BUNDLE") {
        std::fs::write(&path, &built.bytes)?;
        let mut sibling = path;
        sibling.push(".cells.json");
        std::fs::write(sibling, serde_json::to_vec_pretty(&positions)?)?;
    }
    eprintln!(
        "SRV7_S06_NATURAL_FEVERISH_SOURCE {positions} source_cells={}",
        cells.len()
    );
    Ok(())
}
