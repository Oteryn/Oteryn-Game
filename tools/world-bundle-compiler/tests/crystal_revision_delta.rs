//! The Crystal 201189f8 placement delta through the production World Project loader.

use std::collections::BTreeSet;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_spawns};
use oteryn_world_bundle_compiler::source;
use oteryn_world_bundle_compiler::spawn::{BossPolicy, Direction, Period};
use serde::Deserialize;

const SOURCE_PREFIX: &str = "oteryn:spawn.crystal_201189f8.";

#[derive(Deserialize)]
struct Capture {
    groups: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    group: u32,
    centre: (u16, u16, i8),
    points: Vec<DonorPoint>,
}

#[derive(Deserialize)]
struct DonorPoint {
    name: String,
    cell: (u16, u16, i8),
    respawn_s: u32,
    direction: usize,
}

#[derive(Deserialize)]
struct Removals {
    points: Vec<RemovedPoint>,
}

#[derive(Deserialize)]
struct RemovedPoint {
    name: String,
    cell: (u16, u16, i8),
}

#[test]
fn crystal_revision_delta_reaches_the_spawn_table() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let capture: Capture = serde_json::from_str(include_str!(
        "../../../imports/crystalserver/spawn-delta-201189f8.json"
    ))?;
    assert_eq!(capture.groups.len(), 412);
    assert_eq!(
        capture
            .groups
            .iter()
            .map(|group| group.points.len())
            .sum::<usize>(),
        700
    );

    let removed: Removals = serde_json::from_str(include_str!(
        "../../../imports/crystalserver/spawn-removals-201189f8.json"
    ))?;
    assert_eq!(removed.points.len(), 17);
    let regions: BTreeSet<_> = capture
        .groups
        .iter()
        .flat_map(|group| {
            group
                .points
                .iter()
                .map(|point| (point.cell.0 / 256, point.cell.1 / 256))
        })
        .chain(
            removed
                .points
                .iter()
                .map(|point| (point.cell.0 / 256, point.cell.1 / 256)),
        )
        .collect();
    let project = source::project_regions(&root, &regions.into_iter().collect::<Vec<_>>())?;
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
    let dropped: Vec<_> = report
        .dropped
        .iter()
        .filter(|point| point.source.starts_with(SOURCE_PREFIX))
        .collect();
    assert!(
        dropped.is_empty(),
        "Crystal delta points dropped: {dropped:?}"
    );

    let (mut sources, mut points) = (0, 0);
    let mut stag_cells = BTreeSet::new();
    for group in &capture.groups {
        let key = format!("{SOURCE_PREFIX}g{:05}", group.group);
        let compiled = table.sources.iter().find(|source| source.key == key);
        let expected: Vec<_> = group
            .points
            .iter()
            .filter(|point| {
                !matches!(
                    point.name.as_str(),
                    "cobra assassin" | "cobra scout" | "quara kindler" | "rootthing conservator"
                )
            })
            .collect();
        if expected.is_empty() {
            assert!(compiled.is_none(), "held group {key} was realized");
            continue;
        }
        let compiled = compiled.ok_or_else(|| format!("missing donor spawn group {key}"))?;
        let (cx, cy, floor) = group.centre;
        assert_eq!((compiled.x, compiled.y, compiled.floor), (cx, cy, -floor));
        assert_eq!(compiled.points.len(), expected.len(), "{key}");
        for (actual, donor) in compiled.points.iter().zip(expected) {
            let (x, y, floor) = donor.cell;
            let policy = &table.creatures[actual.creature as usize];
            let creature = &policy.key;
            assert_eq!(policy.period, Period::All, "{key}");
            assert_eq!(
                (actual.x, actual.y, compiled.floor),
                (x, y, -floor),
                "{key}"
            );
            assert_eq!(
                *creature,
                format!("oteryn:creature.{}", donor.name.replace(' ', "_"))
            );
            assert_eq!(
                actual.direction,
                [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West
                ][donor.direction]
            );
            if matches!(
                donor.name.as_str(),
                "adlerauge" | "clavius" | "gorga" | "michael the stalwart"
            ) {
                assert_eq!(
                    actual.boss_policy,
                    Some(BossPolicy::Respawn {
                        min_s: u64::from(donor.respawn_s),
                        max_s: u64::from(donor.respawn_s),
                    })
                );
                assert_eq!(actual.respawn_ms, 0);
            } else {
                assert_eq!(actual.boss_policy, None);
                assert_eq!(actual.respawn_ms, donor.respawn_s * 1000);
            }
            if donor.name == "stag" {
                stag_cells.insert((x, y, -floor));
            }
            points += 1;
        }
        sources += 1;
    }
    assert_eq!((sources, points), (386, 674));
    assert_eq!(
        table
            .sources
            .iter()
            .filter(|source| source.key.starts_with(SOURCE_PREFIX))
            .count(),
        sources
    );
    assert_eq!(stag_cells.len(), 15);
    assert!(
        table
            .sources
            .iter()
            .filter(|source| !source.key.starts_with(SOURCE_PREFIX))
            .all(
                |source| source.points.iter().all(|point| !stag_cells.contains(&(
                    point.x,
                    point.y,
                    source.floor
                )))
            ),
        "a predecessor point still occupies a Stag cell"
    );
    for point in removed.points {
        let creature = format!("oteryn:creature.{}", point.name.replace(' ', "_"));
        let (x, y, floor) = point.cell;
        assert!(
            !table.sources.iter().any(|source| {
                source.floor == -floor
                    && source.points.iter().any(|spawn| {
                        spawn.x == x
                            && spawn.y == y
                            && table.creatures[spawn.creature as usize].key == creature
                    })
            }),
            "removed spawn still realized: {creature} at ({x}, {y}, {floor})"
        );
    }
    Ok(())
}
