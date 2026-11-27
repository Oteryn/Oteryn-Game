use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};

#[test]
fn classic_quest_dragonlord_exchange_npc_loads_at_crystal_tower()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    // Crystal 201189f8 world-npc.xml:48–50, the source for the existing exchange.
    let (x, y, z) = (32796, 31558, 2);
    let project = source::project_regions(root, &[(x / 256, y / 256)])?;
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
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let key = "oteryn:npc.an_old_dragonlord";
    let placements = &npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| {
            format!(
                "missing source Dragonlord: {:?}",
                report
                    .held
                    .iter()
                    .filter(|held| held.npc == key)
                    .collect::<Vec<_>>()
            )
        })?
        .placements;
    assert_eq!(placements.len(), 1);
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -z)
    );
    Ok(())
}
