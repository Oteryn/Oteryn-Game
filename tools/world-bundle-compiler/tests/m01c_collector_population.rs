//! Collector routes must have real admitted NPC bodies on retained native map cells.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn m01c_pilgrimage_and_stampede_contacts_realize_on_admitted_native_cells() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let expected: BTreeMap<_, _> = [
        ("oteryn:npc.guide_jonathan", (33181_u16, 31771_u16, 6_i8)),
        ("oteryn:npc.norf", (32345, 32363, 6)),
        ("oteryn:npc.humphrey", (32358, 31688, 6)),
        ("oteryn:npc.edala", (32698, 31718, 2)),
        ("oteryn:npc.kawill", (32643, 31969, 12)),
        ("oteryn:npc.pydar", (32650, 31894, 11)),
        ("oteryn:npc.eremo", (33327, 31882, 7)),
        ("oteryn:npc.guide_tiko", (32543, 32787, 6)),
    ]
    .into_iter()
    .collect();
    let selected: Vec<_> = expected
        .values()
        .map(|&(x, y, _)| (x / 256, y / 256))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut project = source::project_regions(&root, &selected)?;
    project
        .families
        .npc_placements
        .retain(|p| expected.contains_key(p.npc.as_str()));
    assert_eq!(
        project.families.npc_placements.len(),
        expected.len(),
        "all eight quest contacts need physical placements"
    );
    // This case qualifies the contacts, not unrelated global travel destinations.
    project.families.routes.clear();
    for source in &mut project.families.spawns {
        source
            .points
            .retain(|p| selected.contains(&(p.cell.0 / 256, p.cell.1 / 256)));
    }
    project.families.spawns.retain(|s| !s.points.is_empty());
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (npcs, report) = realize_npcs(&input, &registry, &spawns)?;
    assert!(
        report.held.is_empty(),
        "collector placement holds: {report:#?}"
    );
    let realized: BTreeMap<_, _> = npcs
        .npcs
        .iter()
        .flat_map(|npc| {
            npc.placements
                .iter()
                .map(move |p| (npc.key.as_str(), (p.x, p.y, p.floor)))
        })
        .collect();
    let expected_native: BTreeMap<_, _> = expected
        .iter()
        .map(|(&key, &(x, y, z))| (key, (x, y, -z)))
        .collect();
    assert_eq!(realized, expected_native);
    assert_eq!(npcs.placement_count(), 8);
    Ok(())
}
