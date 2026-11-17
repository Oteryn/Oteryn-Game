use super::*;

#[test]
fn m38_minzy_retained_contact_is_admitted_at_its_source_anchor() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let (x, y, z) = (32743, 31971, 12);
    let key = "oteryn:npc.minzy";
    let project =
        oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
    let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
    assert!(
        project
            .families
            .npc_placements
            .iter()
            .any(|p| p.npc == key && p.cell == (x, y, z)),
        "M38 Minzy source placement is not published"
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
    let maris = npcs.npcs.iter().find(|npc| npc.key == key);
    assert!(
        maris.is_some(),
        "Minzy anchor refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == key)
            .collect::<Vec<_>>()
    );
    let placements = &maris.ok_or("admitted Minzy contact")?.placements;
    assert_eq!(placements.len(), 1);
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -(z as i8))
    );
    Ok(())
}
