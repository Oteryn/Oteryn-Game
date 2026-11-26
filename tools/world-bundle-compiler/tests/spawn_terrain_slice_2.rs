use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, SpawnReason, realize_spawns};
use oteryn_world_bundle_compiler::source::{project, registry, world_extent};
use serde::Deserialize;

#[derive(Deserialize)]
struct Capture {
    points: Vec<Anchor>,
}

#[derive(Deserialize)]
struct Anchor {
    source: String,
    creature: String,
    cell: (u16, u16, u8),
    destination: Option<(u16, u16, u8)>,
    #[serde(default)]
    blocked_aquatic: bool,
}

#[test]
fn middle_third_spawn_anchors_are_realized_by_the_world_loader() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = project(&root)?;
    let (registry, palette) = registry(&root, &project)?;
    let Capture { points: anchors } = serde_json::from_str(include_str!(
        "../../../imports/crystalserver/spawn-reconciliation/slice-2.json"
    ))?;
    assert_eq!(anchors.len(), 804);
    assert_eq!(
        anchors
            .iter()
            .map(|row| &row.source)
            .collect::<BTreeSet<_>>()
            .len(),
        547
    );
    let (table, report) = realize_spawns(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    let mut missing = Vec::new();
    let mut blocked = 0;
    for row in anchors {
        if row.blocked_aquatic {
            assert!(report.dropped.iter().any(|drop| {
                drop.source == row.source
                    && drop.creature == row.creature
                    && drop.cell == row.cell
                    && drop.reason == SpawnReason::NoGround
            }));
            blocked += 1;
            continue;
        }
        let (x, y, z) = row.destination.unwrap_or(row.cell);
        let source = table.sources.binary_search_by(|s| s.key.cmp(&row.source));
        if !source.is_ok_and(|i| {
            let group = &table.sources[i];
            group.floor.unsigned_abs() == z
                && group.points.iter().any(|point| {
                    (point.x, point.y) == (x, y)
                        && table.creatures[point.creature as usize].key == row.creature
                })
        }) {
            missing.push((row.source, row.creature, (x, y, z)));
        }
    }
    assert!(
        missing.is_empty(),
        "{} middle-third anchors absent from the loaded spawn table: {:?}",
        missing.len(),
        &missing[..missing.len().min(8)]
    );
    assert_eq!(blocked, 2);
    // Includes Shard of Corruption from Crystal 201189f8
    // data-global/world/world-monster.xml:190614–190616.
    assert_eq!(report.points, 87_820);
    Ok(())
}
