//! Quest reward NPCs must survive real bundle placement and collision admission.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};
use std::{error::Error, path::Path};

#[test]
fn quest_reward_contacts_realize_at_crystal_cells() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (key, (x, y, floor)) in [
        ("oteryn:npc.gnomally", (32795, 31746, -10)),
        ("oteryn:npc.bloodshade_rotten", (34069, 31982, -14)),
        ("oteryn:npc.simon_the_beggar", (32186, 32471, -7)),
    ] {
        let project = source::project_regions(&root, &[(x / 256, y / 256)])?;
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
        let (table, report) = realize_npcs(&input, &registry, &spawns)?;
        let cells: Vec<_> = table
            .npcs
            .iter()
            .filter(|npc| npc.key == key)
            .flat_map(|npc| &npc.placements)
            .map(|cell| (cell.x, cell.y, cell.floor))
            .collect();
        assert_eq!(
            cells,
            [(x, y, floor)],
            "{key}: {:?}",
            report.held.iter().find(|row| row.npc == key)
        );
    }
    Ok(())
}
