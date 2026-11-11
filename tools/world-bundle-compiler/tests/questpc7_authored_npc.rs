//! A declared authored NPC still has to pass the production source-map collision owner.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};
use std::error::Error;

#[test]
fn chosen_grimvale_contacts_pass_native_map_collision_admission() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(129, 126)])?;
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
    for (key, expected) in [
        ("oteryn:npc.rhargu", (33138, 32353, -5)),
        ("oteryn:npc.domizian", (33140, 32353, -5)),
    ] {
        let npc = table
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or_else(|| {
                format!(
                    "contact not realized {key}: {:?}",
                    report
                        .held
                        .iter()
                        .filter(|r| r.npc == key)
                        .collect::<Vec<_>>()
                )
            })?;
        assert_eq!(npc.placements.len(), 1);
        let cell = &npc.placements[0];
        assert_eq!((cell.x, cell.y, cell.floor), expected);
        eprintln!("NATIVE_AUTHORED_NPC_REALIZED {key} {expected:?}");
    }
    Ok(())
}
