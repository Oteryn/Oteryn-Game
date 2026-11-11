use super::*;

#[test]
fn source_stationary_swan_is_admitted_at_its_exact_water_anchor() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let (x, y, z) = (33185, 31732, 7);
    let key = "oteryn:npc.a_swan";
    let project =
        oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
    let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
    assert!(
        project
            .families
            .npc_placements
            .iter()
            .any(|p| p.npc == key && p.cell == (x, y, z)),
        "M38 Swan source placement is not published"
    );
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: oteryn_world_bundle_compiler::source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let swan = npcs.npcs.iter().find(|npc| npc.key == key);
    assert!(
        swan.is_some(),
        "source-stationary Swan refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == key)
            .collect::<Vec<_>>()
    );
    let placements = &swan.ok_or("admitted stationary Swan")?.placements;
    assert_eq!(placements.len(), 1);
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -(z as i8))
    );
    Ok(())
}
