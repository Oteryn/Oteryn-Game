//! A declared authored NPC still has to pass the production source-map collision owner.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};
use std::error::Error;

#[test]
fn authored_lai_cell_is_realized_from_the_native_newhaven_map() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(127, 126)])?;
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
    let npc = table
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.lai")
        .ok_or_else(|| {
            format!(
                "authored Lai not realized: {:?}",
                report
                    .held
                    .iter()
                    .filter(|row| row.npc == "oteryn:npc.lai")
                    .collect::<Vec<_>>()
            )
        })?;
    assert_eq!(npc.placements.len(), 1);
    let cell = &npc.placements[0];
    assert_eq!((cell.x, cell.y, cell.floor), (32571, 32506, -7));
    eprintln!("NATIVE_AUTHORED_NPC_REALIZED lai x32571 y32506 floor-7");
    Ok(())
}
