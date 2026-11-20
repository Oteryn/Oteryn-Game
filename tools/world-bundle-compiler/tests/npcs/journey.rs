use std::path::Path;

use oteryn_world_bundle_compiler::source;

use super::*;

#[test]
fn journey_npcs_compile_at_their_crystal_cells() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = source::project_regions(root, &[(124, 124), (126, 125), (129, 124), (129, 126)])?;
    let (resolver, palette) = source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;

    let expected = [
        ("oteryn:npc.alexander", (33256, 31839, -3)),
        ("oteryn:npc.asima", (33220, 32403, -7)),
        ("oteryn:npc.aurelia", (31955, 31916, -7)),
        ("oteryn:npc.sam", (32361, 32198, -7)),
    ];
    let missing: Vec<_> = expected
        .into_iter()
        .filter(|(key, cell)| {
            !table.npcs.iter().any(|npc| {
                npc.key == *key
                    && npc.placements.len() == 1
                    && (
                        npc.placements[0].x,
                        npc.placements[0].y,
                        npc.placements[0].floor,
                    ) == *cell
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "missing Crystal placements: {missing:?}; placement holds: {:?}",
        report
            .held
            .iter()
            .filter(|hold| missing.iter().any(|(key, _)| hold.npc == *key))
            .collect::<Vec<_>>()
    );
    Ok(())
}
