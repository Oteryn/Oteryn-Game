//! Check the reconciled Crystal anchors against the normal World source loader.
use std::collections::BTreeSet;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, SpawnReason, realize_spawns};
use oteryn_world_bundle_compiler::source;
use serde::Deserialize;

#[derive(Deserialize)]
struct Reconciliation {
    last_source_key: String,
    points: Vec<Point>,
}

#[derive(Deserialize)]
struct Point {
    source: String,
    creature: String,
    to: (u16, u16, u8),
}

#[test]
fn slice_one_reconciled_points_realize_from_world_sources() -> Result<(), Box<dyn std::error::Error>>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let changes: Reconciliation =
        serde_json::from_str(include_str!("../spawn-reconciliation-1.json"))?;
    let project = source::project(&root)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (table, report) = realize_spawns(
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
    let mut realized = BTreeSet::new();
    for source in &table.sources {
        for point in &source.points {
            let creature = &table.creatures[point.creature as usize].key;
            realized.insert((
                source.key.as_str(),
                creature.as_str(),
                (point.x, point.y, source.floor.unsigned_abs()),
            ));
        }
    }
    assert_eq!(changes.points.len(), 419);
    let missing: Vec<_> = changes
        .points
        .iter()
        .filter(|point| {
            !realized.contains(&(point.source.as_str(), point.creature.as_str(), point.to))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{} expected points absent, first is {} at {:?}",
        missing.len(),
        missing.first().map_or("", |point| point.source.as_str()),
        missing.first().map(|point| point.to)
    );

    // Includes Crystal 201189f8 Shard of Corruption from world-monster.xml:190614–190616.
    assert_eq!((report.sources, report.points), (55_075, 88_477));
    assert_eq!(report.realized_points, 86_402);
    assert_eq!(report.dropped.len(), 2_075);
    for (reason, expected) in [
        (SpawnReason::NoGround, 2),
        (SpawnReason::NotWalkable, 0),
        (SpawnReason::ProtectionZone, 0),
    ] {
        assert_eq!(
            report
                .dropped
                .iter()
                .filter(|point| point.source > changes.last_source_key && point.reason == reason)
                .count(),
            expected,
            "later slice changed for {reason:?}"
        );
    }
    Ok(())
}
