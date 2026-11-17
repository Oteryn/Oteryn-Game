use super::*;

#[test]
fn m38_zarifan_contact_is_admitted_at_its_source_anchor() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let (x, y, z) = (33040, 32484, 11);
    let key = "oteryn:npc.zarifan";
    let project =
        oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
    let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
    assert!(
        project
            .families
            .npc_placements
            .iter()
            .any(|p| p.npc == key && p.cell == (x, y, z)),
        "M38 Zarifan source placement is not published"
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
    let zarifan = npcs.npcs.iter().find(|npc| npc.key == key);
    assert!(
        zarifan.is_some(),
        "Zarifan anchor refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == key)
            .collect::<Vec<_>>()
    );
    let placements = &zarifan.ok_or("admitted Zarifan contact")?.placements;
    assert_eq!(placements.len(), 1);
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -(z as i8))
    );
    let mut ordinary_families = project.families.clone();
    ordinary_families.npc_block_solid_contacts.clear();
    let ordinary = Input {
        families: &ordinary_families,
        ..input
    };
    let (ordinary_npcs, report) = realize_npcs(&ordinary, &resolver, &spawns)?;
    assert!(!ordinary_npcs.npcs.iter().any(|npc| npc.key == key));
    assert!(report.held.iter().any(|held| held.npc == key
        && held.reason == oteryn_world_bundle_compiler::compile::HoldReason::BlockSolid));
    Ok(())
}
