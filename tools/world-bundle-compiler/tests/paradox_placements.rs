use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::{
    compile::{Input, realize_npcs, realize_spawns},
    source,
};

#[test]
fn paradox_contacts_realize_at_crystal_centers() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = source::project_regions(root, &[(126, 124), (126, 125), (128, 126), (129, 124)])?;
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

    for (name, cell) in [
        ("a_prisoner", (32394, 32138, -13)),
        ("lubo", (32489, 32119, -7)),
        ("oldrak", (32817, 32261, -6)),
        ("padreia", (32302, 31814, -7)),
        ("riddler", (32479, 31901, -2)),
        ("zoltan", (33266, 31847, -4)),
    ] {
        let key = format!("oteryn:npc.{name}");
        assert!(
            table
                .npcs
                .iter()
                .any(|npc| npc.key == key
                    && npc.placements.iter().any(|p| (p.x, p.y, p.floor) == cell)),
            "{key} missing at {cell:?}; holds: {:?}",
            report
                .held
                .iter()
                .filter(|hold| hold.npc == key)
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}
