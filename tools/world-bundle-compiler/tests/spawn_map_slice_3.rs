use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_spawns};
use oteryn_world_bundle_compiler::source::{project_regions, registry, world_extent};
use serde::Deserialize;

#[derive(Deserialize)]
struct Capture {
    points: Vec<Point>,
}

#[derive(Deserialize)]
struct Point {
    source: String,
    creature: String,
    cell: (u16, u16, u8),
    move_to: Option<(u16, u16, u8)>,
}

#[test]
fn slice_three_spawn_points_load_at_donor_qualified_cells() -> Result<(), Box<dyn Error>> {
    let capture: Capture = serde_json::from_str(include_str!("../spawn-map-slice-3.json"))?;
    assert_eq!(capture.points.len(), 709);
    let selected: BTreeSet<_> = capture
        .points
        .iter()
        .map(|point| {
            let (x, y, _) = point.move_to.unwrap_or(point.cell);
            (x / 256, y / 256)
        })
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut project = project_regions(&root, &selected.into_iter().collect::<Vec<_>>())?;
    let sources: BTreeSet<_> = capture
        .points
        .iter()
        .map(|point| point.source.as_str())
        .collect();
    assert_eq!(sources.len(), 547);
    project
        .families
        .spawns
        .retain(|source| sources.contains(source.key.as_str()));
    let (registry, palette) = registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (table, _) = realize_spawns(&input, &registry)?;
    let loaded: BTreeMap<_, _> = table
        .sources
        .iter()
        .map(|source| (source.key.as_str(), source))
        .collect();
    let mut missing = Vec::new();
    for point in &capture.points {
        let (x, y, z) = point.move_to.unwrap_or(point.cell);
        let present = loaded.get(point.source.as_str()).is_some_and(|source| {
            i16::from(source.floor) == -i16::from(z)
                && source.points.iter().any(|candidate| {
                    (candidate.x, candidate.y) == (x, y)
                        && table.creatures[candidate.creature as usize].key == point.creature
                })
        });
        if !present {
            missing.push((&point.source, &point.creature, (x, y, z)));
        }
    }
    assert!(
        missing.is_empty(),
        "{} donor-qualified points missing, first five: {:?}",
        missing.len(),
        &missing[..missing.len().min(5)]
    );
    Ok(())
}
