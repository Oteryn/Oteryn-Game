//! Exact ordinary source placements for six S06 NPC operands and charged dracoyle.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_acquisition_seven_natural_producers_export_actual_source_bundle() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let names = [
        "bear",
        "chicken",
        "hellspawn",
        "minotaur",
        "rotworm",
        "wasp",
        "wolf",
    ];
    let mut points: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut files = std::fs::read_dir(root.join("content/world/spawns"))?
        .map(|p| p.map(|p| p.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.sort();
    for path in files.iter().filter(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("spawns-") && n.ends_with(".json"))
    }) {
        let raw = std::fs::read(path)?;
        let shard: Value = serde_json::from_slice(&raw)?;
        for (ri, r) in shard["records"]
            .as_array()
            .ok_or("spawn records")?
            .iter()
            .enumerate()
        {
            for (pi, p) in r["declaration"]["points"]
                .as_array()
                .ok_or("spawn points")?
                .iter()
                .enumerate()
            {
                let Some(key) = p["creature"].as_str() else {
                    continue;
                };
                let Some(name) = key.strip_prefix("oteryn:creature.") else {
                    continue;
                };
                if !names.contains(&name) {
                    continue;
                }
                points.entry(name.to_owned()).or_default().push(json!({"cell":p["cell"],"path":path.strip_prefix(&root)?.to_string_lossy(),"json_path":format!("$.records[{ri}].declaration.points[{pi}]"),"spawn":r["declaration"]["identity"]["key"]}));
            }
        }
    }
    let mut regions = BTreeSet::new();
    for name in names {
        let p = points
            .get(name)
            .and_then(|p| p.first())
            .ok_or("ordinary natural producer placement")?;
        regions.insert((
            u16::try_from(p["cell"]["x"].as_u64().ok_or("x")? / 256)?,
            u16::try_from(p["cell"]["y"].as_u64().ok_or("y")? / 256)?,
        ));
    }
    let project = source::project_regions(&root, &regions.iter().copied().collect::<Vec<_>>())?;
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
    let mut cells = BTreeSet::new();
    for s in &data.sectors {
        for t in &s.tiles {
            if t.flags == 0
                && t.house == 0
                && t.zones.is_empty()
                && t.items.len() == 1
                && t.items[0].depth == 0
                && data.manifest.palette[t.items[0].palette as usize]
                    .terrain
                    .is_some_and(|p| p.kind == TerrainKind::Ground && p.walkable == Some(true))
            {
                cells.insert((i32::from(t.x), i32::from(t.y), i16::from(-s.floor)));
            }
        }
    }
    let mut selected = BTreeMap::new();
    for name in names {
        let found=points.get(name).ok_or("producer source")?.iter().find_map(|source|{
   let p=&source["cell"];let x=i32::try_from(p["x"].as_u64()?).ok()?;let y=i32::try_from(p["y"].as_u64()?).ok()?;let z=i16::try_from(p["floor"].as_i64()?).ok()?;
   if !cells.contains(&(x,y,z)){return None;}
   [(x+1,y,z),(x-1,y,z),(x,y+1,z),(x,y-1,z)].into_iter().find(|p|cells.contains(p)).map(|p|json!({"creature":format!("oteryn:creature.{name}"),"player":[p.0,p.1,p.2],"target":[x,y,z],"source":source}))
  }).ok_or_else(||format!("actual walkable source placement/adjacent player for{name}"))?;
        selected.insert(name.to_owned(), found);
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_ACQUISITION_BUNDLE") {
        std::fs::write(&path, &built.bytes)?;
        let mut sibling = path;
        sibling.push(".cells.json");
        std::fs::write(sibling, serde_json::to_vec_pretty(&selected)?)?;
    }
    eprintln!(
        "SRV7_ACQUISITION_NATURAL_PRODUCERS_SOURCE count=7 unchanged_b3=true unchanged_native_loot=true regions={regions:?} sources={selected:?}"
    );
    Ok(())
}
